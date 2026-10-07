# Testing

Following [matklad's testing philosophy](https://matklad.github.io/2021/05/31/how-to-test.html), crawfish uses **integrated snapshot tests** as its primary testing strategy. Compilers are pure self-contained functions (source in, structured output), which makes them ideal for this approach.

Each test feeds a `.bb` source file through the full pipeline up to some stage, snapshots the output, and compares it against a saved baseline. It's great because it tests features: the tests are independent of internal APIs, so refactoring internals doesn't break them. Adding a new test case is just adding a new `.bb` file.

Property-based tests are reserved for isolated algorithmic code (e.g., the `HandleList`, `InferenceTable`).

## Property-Based Testing

To [property-based test](https://tigerbeetle.com/blog/2025-04-23-swarm-testing-data-structures/),

## Snapshot Testing

To [snapshot test](https://www.cs.cornell.edu/~asampson/blog/turnt.html), blueberry uses [insta](https://github.com/mitsuhiko/insta). Each compilation "stage" has a single test that globs over all `.crw` input files in its `inputs/` directory, runs the pipeline, and snapshots the result.

1. Create a blueberry file `.bb` in `<stage>/snapshot_inputs/`.

2. Add the following at the bottom of what you want to test (usually the compiler stage's state director):

```rust
[cfg(test)]
mod tests {
    use super::*;
    use crate::core::lexical_analysis::tokenizer::Tokenizer;
    use crate::core::syntactic_analysis::cst::cst_builder::CstBuilder;
    use std::fs;

    #[test]
    fn test_STATE_MANAGER_output() {
        insta::glob!("snapshot_inputs/**/*.bb", |path| {
            let input = fs::read_to_string(path).unwrap();
            let (tokens, _diagnostics) = Tokenizer::new(&input).tokenize();

            let (sink, diagnostics) = Parser::new(&tokens).parse();
            let (cst, diagnostics) = CstBuilder::new(&input, &tokens, sink, diagnostics).build();


            // ...

            insta::assert_snapshot!(dump);
        })
    }
}
```

2. Run the test (it will fail because no snapshot exists yet).

3. Review the snapshot using `cargo insta review` to make sure the output looks correct. If it does, accept it.

4. Commit the dump output's file (`.snap`) to git. This is what makes it a regression test going forward.