use std::collections::HashMap;
use std::fmt::{self, Write};

use handlemap::handle_map::Handle;

use crate::core::common::context::CompilerContext;
use crate::middle_end::mir::{BlockId, Function, InstructionId, InstructionRef, Mir, ValueId};

pub(crate) struct MirDumper<'a> {
    mir: &'a Mir,
    ctx: &'a CompilerContext,
}

impl<'a> MirDumper<'a> {
    pub(crate) fn new(mir: &'a Mir, ctx: &'a CompilerContext) -> Self {
        Self { mir, ctx }
    }

    pub(crate) fn dump(&self) -> Result<String, fmt::Error> {
        let mut out = String::new();
        for function in self.mir.functions() {
            out.push_str(&FunctionDumper::new(function, self.ctx).dump()?);
            out.push('\n');
        }
        Ok(out)
    }
}

pub(crate) trait FunctionWriter {
    fn write_block_header(
        &mut self,
        dumper: &FunctionDumper,
        out: &mut String,
        block: BlockId,
        indent: usize,
    ) -> fmt::Result {
        dumper.block_header(out, block, indent)
    }

    fn write_instruction(
        &mut self,
        dumper: &FunctionDumper,
        out: &mut String,
        instruction: InstructionId,
        indent: usize,
    ) -> fmt::Result {
        dumper.instruction(out, instruction, indent)
    }
}

pub(crate) struct PlainWriter;

impl FunctionWriter for PlainWriter {}

pub(crate) struct FunctionDumper<'a> {
    function: &'a Function,
    ctx: &'a CompilerContext,
}

impl<'a> FunctionDumper<'a> {
    pub(crate) fn new(function: &'a Function, ctx: &'a CompilerContext) -> Self {
        Self { function, ctx }
    }

    pub(crate) fn dump(&self) -> Result<String, fmt::Error> {
        self.dump_with(&mut PlainWriter)
    }

    pub(crate) fn dump_with<FW: FunctionWriter>(
        &self,
        writer: &mut FW,
    ) -> Result<String, fmt::Error> {
        let mut out = String::new();
        self.decorate_function(writer, &mut out)?;
        Ok(out)
    }

    pub(crate) fn signature_line(&self) -> String {
        let name = self
            .ctx
            .string_interner
            .resolve(self.function.name)
            .unwrap();
        let parameters = self
            .function
            .signature
            .parameter_type_ids
            .iter()
            .map(|&ty| self.ctx.type_interner.to_string(ty))
            .collect::<Vec<_>>()
            .join(", ");
        let return_type = self
            .ctx
            .type_interner
            .to_string(self.function.signature.return_type_id);
        format!("{name}({parameters}) -> {return_type}")
    }

    fn decorate_function<FW: FunctionWriter>(
        &self,
        writer: &mut FW,
        out: &mut String,
    ) -> fmt::Result {
        // spec line
        writeln!(out, "function {} {{", self.signature_line())?;

        // Instructions indent 4; block headers sit outdented 4 from that.
        let indent = 4;

        // immediate-target → values aliased directly to it
        let mut aliases = self.alias_map();

        // Iterate the layout, not the dfg — block order is the layout's
        // business.
        let mut first = true;
        for block in self.function.body.block_ids() {
            if !first {
                writeln!(out)?;
            }
            first = false;
            self.decorate_block(writer, out, &mut aliases, block, indent)?;
        }

        // Aliases whose target has no printed definition site (e.g. a
        // trivial merge resolved to an `Undefined` placeholder).
        if !aliases.is_empty() {
            writeln!(out)?;
            writeln!(
                out,
                "{:1$}; aliases of otherwise-undefined values",
                "",
                indent.saturating_sub(4)
            )?;
            let mut targets: Vec<ValueId> = aliases.keys().copied().collect();
            targets.sort_by_key(|value| value.index());
            for target in targets {
                self.write_value_aliases(out, &mut aliases, target, indent)?;
            }
        }

        writeln!(out, "}}")
    }

    fn decorate_block<FW: FunctionWriter>(
        &self,
        writer: &mut FW,
        out: &mut String,
        aliases: &mut HashMap<ValueId, Vec<ValueId>>,
        block: BlockId,
        indent: usize,
    ) -> fmt::Result {
        writer.write_block_header(self, out, block, indent)?;

        for &parameter in self.function.body.get_block(block).parameters() {
            self.write_value_aliases(out, aliases, parameter, indent)?;
        }

        for instruction in self.function.body.get_block(block).instructions() {
            writer.write_instruction(self, out, instruction, indent)?;
            for &result in self.function.body.get_instruction(instruction).results() {
                self.write_value_aliases(out, aliases, result, indent)?;
            }
        }
        Ok(())
    }

    pub(crate) fn block_header(
        &self,
        out: &mut String,
        block: BlockId,
        indent: usize,
    ) -> fmt::Result {
        write!(out, "{:1$}Block{2}", "", indent - 4, block.index())?;

        let parameters = self.function.body.get_block(block).parameters();
        if !parameters.is_empty() {
            let rendered = parameters
                .iter()
                .map(|&parameter| {
                    let ty = self.function.body.get_value(parameter).ty();
                    format!(
                        "v{}: {}",
                        parameter.index(),
                        self.ctx.type_interner.to_string(ty)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            write!(out, "({rendered})")?;
        }
        writeln!(out, ":")
    }

    pub(crate) fn instruction(
        &self,
        out: &mut String,
        instruction: InstructionId,
        indent: usize,
    ) -> fmt::Result {
        write!(out, "{:indent$}", "")?;

        let view = self.function.body.get_instruction(instruction);

        let results = view.results();
        if !results.is_empty() {
            write!(out, "{} = ", join_values(results))?;
        }

        match view.as_instruction_ref() {
            InstructionRef::Binary {
                operator,
                operand_ids,
            } => {
                write!(
                    out,
                    "Binary {:?} v{}, v{}",
                    operator,
                    operand_ids[0].index(),
                    operand_ids[1].index()
                )?;
            }
            InstructionRef::Unary {
                operator,
                operand_id,
            } => {
                write!(out, "Unary {:?} v{}", operator, operand_id.index())?;
            }
            InstructionRef::IntegerLiteral { value } => {
                write!(out, "IntegerLiteral {value}")?;
            }
            InstructionRef::BooleanLiteral { value } => {
                write!(out, "BooleanLiteral {value}")?;
            }
            InstructionRef::Call {
                callee_id,
                argument_ids,
            } => {
                let name = self
                    .ctx
                    .string_interner
                    .resolve(self.function.body.get_function_reference(callee_id).name)
                    .unwrap();
                write!(out, "Call {name}({})", join_values(argument_ids))?;
            }
            InstructionRef::Jump {
                destination_id,
                block_argument_ids,
            } => {
                write!(out, "Jump ")?;
                self.block_call(out, destination_id, block_argument_ids)?;
            }
            InstructionRef::ConditionalBranch {
                operand_id,
                true_block_id,
                true_block_argument_ids,
                false_block_id,
                false_block_argument_ids,
            } => {
                write!(out, "ConditionalBranch v{}, ", operand_id.index())?;
                self.block_call(out, true_block_id, true_block_argument_ids)?;
                write!(out, ", ")?;
                self.block_call(out, false_block_id, false_block_argument_ids)?;
            }
            InstructionRef::Return { output_ids } => {
                if output_ids.is_empty() {
                    write!(out, "Return")?;
                } else {
                    write!(out, "Return {}", join_values(output_ids))?;
                }
            }
            InstructionRef::Unreachable => write!(out, "Unreachable")?,
        }
        writeln!(out)
    }

    fn block_call(&self, out: &mut String, block: BlockId, args: &[ValueId]) -> fmt::Result {
        write!(out, "Block{}", block.index())?;
        if !args.is_empty() {
            write!(out, "({})", join_values(args))?;
        }
        Ok(())
    }

    fn alias_map(&self) -> HashMap<ValueId, Vec<ValueId>> {
        let mut map: HashMap<ValueId, Vec<ValueId>> = HashMap::new();
        for value in self.function.body.ssa_ids() {
            if let Some(target) = self.function.body.get_value(value).alias_target() {
                map.entry(target).or_default().push(value);
            }
        }
        map
    }

    fn write_value_aliases(
        &self,
        out: &mut String,
        aliases: &mut HashMap<ValueId, Vec<ValueId>>,
        target: ValueId,
        indent: usize,
    ) -> fmt::Result {
        let mut todo_stack = vec![target];
        while let Some(target) = todo_stack.pop() {
            if let Some(list) = aliases.remove(&target) {
                for alias in list {
                    writeln!(
                        out,
                        "{:1$}v{2} -> v{3}",
                        "",
                        indent,
                        alias.index(),
                        target.index()
                    )?;
                    todo_stack.push(alias);
                }
            }
        }
        Ok(())
    }
}

fn join_values(values: &[ValueId]) -> String {
    values
        .iter()
        .map(|value| format!("v{}", value.index()))
        .collect::<Vec<_>>()
        .join(", ")
}
