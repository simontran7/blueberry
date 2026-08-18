use std::fmt::{self, Write};

use soup::handle_map::Handle;

use crate::common::context::CompilerContext;
use crate::middle_end::mir::{Function, InstructionRef, Mir};
use crate::middle_end::mir_dumper::FunctionDumper;

pub(crate) struct DotDumper<'a> {
    mir: &'a Mir,
    ctx: &'a CompilerContext,
}

impl<'a> DotDumper<'a> {
    pub(crate) fn new(mir: &'a Mir, ctx: &'a CompilerContext) -> Self {
        Self { mir, ctx }
    }

    pub(crate) fn dump(&self) -> Result<String, fmt::Error> {
        let mut out = String::new();
        writeln!(out, "digraph mir {{")?;
        // Sibling function clusters share no edges (there's no inter-procedural
        // call edge, only intra-function control flow), so `dot` has no
        // constraint keeping them apart — without generous spacing here,
        // unrelated clusters can end up crowded, even overlapping.
        writeln!(out, "    nodesep=0.5;")?;
        writeln!(out, "    ranksep=0.75;")?;
        writeln!(out, "    node [shape=box, fontname=monospace];")?;
        for function in self.mir.functions() {
            self.dump_function(&mut out, function)?;
        }
        writeln!(out, "}}")?;
        Ok(out)
    }

    fn dump_function(&self, out: &mut String, function: &Function) -> fmt::Result {
        let name = self
            .ctx
            .string_interner
            .resolve(function.name)
            .expect("function name symbol not interned");
        let dumper = FunctionDumper::new(function, self.ctx);

        writeln!(out, "    subgraph \"cluster_{name}\" {{")?;
        writeln!(out, "        label=\"{name}\";")?;
        writeln!(out, "        margin=16;")?;

        // Classical CFG convention: explicit ENTRY/EXIT sentinel nodes
        // bracketing the graph, distinct from the block rectangles —
        // an edge from ENTRY into the first real block, and an edge from
        // every `return` into a single shared EXIT.
        let entry_id = format!("{name}_ENTRY");
        let exit_id = format!("{name}_EXIT");
        writeln!(
            out,
            "        \"{entry_id}\" [shape=ellipse, label=\"ENTRY\"];"
        )?;
        writeln!(
            out,
            "        \"{exit_id}\" [shape=ellipse, label=\"EXIT\"];"
        )?;
        let first_block = function
            .body
            .block_ids()
            .next()
            .expect("function has no blocks");
        writeln!(
            out,
            "        \"{entry_id}\" -> \"{name}_Block{}\";",
            first_block.index()
        )?;

        for block in function.body.block_ids() {
            let node_id = format!("{name}_Block{}", block.index());

            let mut label = String::new();
            dumper.block_header(&mut label, block, 4)?;
            for instruction in function.body.get_block(block).instructions() {
                dumper.instruction(&mut label, instruction, 4)?;
            }
            // Some DOT viewers size a box from font metrics that don't match
            // what they actually render (monospace often isn't resolved the
            // same way for measuring vs. drawing), leaving the box too
            // narrow for its own text. Sizing it explicitly from the
            // longest line sidesteps that mismatch entirely.
            let longest_line = label.lines().map(str::len).max().unwrap_or(0);
            let width = (longest_line as f64 * 0.11).max(1.0);
            writeln!(
                out,
                "        \"{node_id}\" [label=\"{}\", width={width:.2}];",
                escape(&label)
            )?;

            // Edges: every block ends in exactly one terminator by
            // construction, so `last_instruction` is always it.
            let terminator = function
                .body
                .get_block(block)
                .last_instruction()
                .expect("block has no terminator");
            match function
                .body
                .get_instruction(terminator)
                .as_instruction_ref()
            {
                InstructionRef::Jump { destination_id, .. } => {
                    writeln!(
                        out,
                        "        \"{node_id}\" -> \"{name}_Block{}\";",
                        destination_id.index()
                    )?;
                }
                InstructionRef::ConditionalBranch {
                    true_block_id,
                    false_block_id,
                    ..
                } => {
                    writeln!(
                        out,
                        "        \"{node_id}\" -> \"{name}_Block{}\" [label=\"true\"];",
                        true_block_id.index()
                    )?;
                    writeln!(
                        out,
                        "        \"{node_id}\" -> \"{name}_Block{}\" [label=\"false\"];",
                        false_block_id.index()
                    )?;
                }
                InstructionRef::Return { .. } => {
                    writeln!(out, "        \"{node_id}\" -> \"{exit_id}\";")?;
                }
                InstructionRef::Unreachable => {}
                _ => unreachable!("non-terminator as last instruction in block"),
            }
        }
        writeln!(out, "    }}")
    }
}

fn escape(label: &str) -> String {
    label
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\l")
}
