# SSA Construction Algorithm

The [initial SSA construction algorithm](https://dl.acm.org/doi/pdf/10.1145/75277.75280) accepts a CFG as input, and works as follows:
1. Computes for every block $X$ in the CFG its **dominance frontier** $DF$. A dominance frontier is the set of all blocks $Y$ where $X$ dominates one of its predecessors, *but* $X$ does *not* dominate $Y$. 

For instance: in the following CFG, $DF(B) = {D}$ (i.e., the dominance frontier of block $B$ is only $D$) since $D$'s predecessor $E$ is dominated by $B$, yet $B$ does not dominate $D$ since there's a path to $D$ from $C$.

```text
   / \
  B   C
  |   |
  E   |
   \ /
    D
```

2. For each variable, find every block that contains an assignment of it, union their dominance frontiers, then put a φ-node in each block of that union. This is because every block in a dominance frontier is a merge point! 

> [!NOTE]
> Facts about dominances
> - A block $A$ is said to dominate a block $B$ if every path from the entry block of the CFG to block $B$ passes through block $A$. A block $A$ strictly dominates a block $B$ if the block $A$ dominates block $B$ and $A \neq B$
> - Every block dominates (but does not strictly dominate) itself.

This indicates where to create and insert phi nodes.

3. Rename variables to ensure SSA's single assignment property is satisfied.

However, Cytron et al.'s algorithm pays two costs before a single phi node is placed: the AST must already be lowered to a CFG, and the dominance frontier (typically alongside the dominator tree) must be computed for the *entire* CFG upfront, regardless of how many variables actually need phi nodes. 

This is where [Braun et al.'s algorithm](https://link.springer.com/chapter/10.1007/978-3-642-37051-9_6) comes in. It lowers straight from the typed IR to SSA (skipping the dominance frontier analysis entirely from Cytron et al.'s algorithm), by placing phi nodes lazily via recursion instead of computing them all upfront:
- Base Case (**Local value numbering**): check if the variable was already assigned earlier in the same block, and if so, just reuse that value directly (since there's only ever one possible path that led to that assignment executing: the one you're already on)
- Recursive Step (**Global value numbering**): if a block currently contains no definition for a variable, we recursively look for a definition in its predecessors. Which of three cases applies depends on the block's sealed status and predecessor count:
    - **Unsealed** (not all predecessors known yet): create an empty phi node for this block as a placeholder.

> [!NOTE]
> Sealing (`declare_block` then, later, `seal_block`) is an explicit, caller-driven action: seal a block the moment its predecessor set is final. Most blocks know that upfront and seal immediately; loop headers don't, since the back-edge doesn't exist until the whole body is lowered, so sealing waits until then.

    - **Sealed, single predecessor**: skip creating a phi node entirely, and just query that one predecessor recursively for a definition instead since there's only one possible path into this block, and so there's nothing to merge.
    - **Sealed, multiple predecessors**: create an empty phi node for this block first to prevent infinite recursion (the placeholder is what a reentrant lookup for the same block finds instead of recursing forever, breaking the cycle), record it as the current definition for the variable in the block, then recurse into every predecessor and ask each for their value.
        - If all predecessors give the same value: no phi node is needed at all. That single value is the answer, and just hand it back up.
        - If the predecessors give different values: that disagreement means this block is a genuine merge point, so the placeholder phi node's operands get filled in, one operand per predecessor, matching each predecessor's value to its corresponding edge. The phi node's result value becomes the answer.
        - Then, check whether that phi node is *trivial* i.e., if its operands, once filled in, all turn out to be the same value (ignoring any operand that just points back to the phi node itself), and thus, not merging anything. This phi node is therefore removed, and every use of it is replaced with that shared value directly. Additionally, a phi node is considered trivial if the phi node has no operands besides itself, it means that it can't actually be reached with from any predecessor (i.e., it's either dead/unreachable code, or it's the function's entry block, as the entry block has no predecessors at all) either unreachable or in the start block. Since there's nothing sensible to substitute, we plug in an explicit **undefined** placeholder value as the phi node's replacement, so it takes the phi node's place wherever the phi was already being used. 

> [!NOTE]
> This fill-then-check order only stays cheap for classical phi nodes whose operands live on the phi node itself. For **block parameters**, operands instead live on each predecessor's jump or branch instruction as block arguments, so filling them in *before* checking triviality means a trivial result leaves the block's parameter count out of sync with its predecessors' argument count, forcing the one argument added to be stripped back out of every predecessor. Cranelift avoids this by checking triviality first, before writing any arguments, and only committing them once a parameter is known to survive (i.e., there's no process of removing block arguments).

> [!NOTE]
> It is necessary to *recursively* remove trivial phi nodes as other phi nodes elsewhere may hold the now-deleted trivial phi node as one of their operands. Once that operand is rewritten to the common value $v$, those phis' operand lists change too, which can newly make *them* trivial so the check has to cascade to every user of the removed phi, and not *just* the phi itself. However, this code doesn't do that, but only **aliases** trivial block parameters, then rewrites them once, in a single batch at the end of construction (`flush_aliases()`). 

Consider the following crawfish source program:

```
let x = ...;
while ... {
    if ... {    
        x = ...;
    }   
}
println(x);
```

Braun's algorithm would tackle the SSA construction as follows (assuming the loop is constructed before `x` is read):

1. `let x = ...;` and `x = ...;` are both simple assignments. We record for `let x = ...;` as `v0`, and `x = ...;` inside the if expression as `v1`.

<img src="./step-1-state.png" width="350">

2. For `println(x);`, it does not contain a local definition of `x`, so we recurse upwards to its single predecessor block $B$ (this is example of the fast path executing), requesting for the definition of `x`.

3. Now at block $B$, we check if it has a local definition of `x`. It does not. But block $B$ has two predecessors: block $A$ (entering the loop the first time) and block $F$ (coming back around after one iteration). With no location definition in the current block $B$, but two predecessors, it is a merge point, so we create an empty phi node labeled $v2$ for the block $B$, and immediately register $v2$ as block $B$'s current definition of `x`. Then, we recurse into block $B$'s two predecessors to fill in $v2$'s operands.

<img src="./step-3-state.png" width="350">

4. In block $A$, there exists a local definition of `x` labeled $v0$ (created in step 1) and return $v0$ so that it may become $v2$'s first operand. In block $F$, there are no local definitions of `x`, but block $F$ has two predecessors: block $D$ and block $E$. This signals that it also a merge point, and so, we create an empty phi node $v3$, and register it as block $F$'s local definition of `x`. We now recurse into block $F$'s predecessors (block $D$ and block $E$).

<img src="./step-4-state.png" width="350">

5. In block $D$, there is a local definition of `x` labeled $v1$ (created in step 1), so we return $v1$ so that it may become $v3$'s first operand. In block $E$, there are unfortunately no local definitions of `x`. It does have one predecessor: block $C$, so we don't need to create a phi node, and we recurse into block $C$.

<img src="./step-5-state.png" width="350">

6. In block $C$, it also has no local definitions of `x`, but it has one predecessor: block $B$, so we recurse once more without having to create a phi node.
7. In block $B$, we finally see a local definition of `x` labeled $v2$, which was created in step 3. Had we not created that empty phi node, we would have done recurse down the same path, on and on, recursing infinitely! We return $v2$ thrice back down to the stack frame created in step 4 so that it may become $v3$'s second operand.

<img src="./step-7-state.png" width="350">

8. In the current stack frame for step 4, we perform another return to pass down $v3$ — a filled phi node with operands $v1$ and $v2$ — as a second operand of the phi node $v2$ created in step 3.

<img src="./step-8-state.png" width="350">

9. We now have completed block B's $v2$ phi node. It has as first operand $v0$, and as second operand $v3$.

> [!NOTE]
> Since this Braun et al's algorithm doesn't build a dominance frontier, any later pass that may require one (e.g. loop-invariant code motion, contification) must compute it separately, which isn't much different than the upfront dominance frontier compute cost of Cytron et al. algorithm.

> [!NOTE]
> Braun et al.'s algorithm also enables on-the-fly local optimizations (constant folding, copy propagation, common subexpression elimination) during construction, since values are built incrementally anyway.



