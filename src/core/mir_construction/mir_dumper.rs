use std::collections::HashMap;
use std::fmt::{self, Write};

use crate::core::common::handle_collections::Handle;
use crate::core::mir_construction::mir::{
    BlockId, InstructionId, InstructionRef, MirBody, ValueId,
};
use crate::core::mir_construction::{constant_mir_of, function_mir_of};
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::semantic_analysis::{
    all_definitions_of, constant_signature_of, function_signature_of,
};
use crate::core::source_file_key::SourceFileKey;

pub(crate) struct MirBodyDumper<'a, 'db> {
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
    body: &'a MirBody<'db>,
}

pub(crate) trait BodyWriter {
    fn write_block_header(
        &mut self,
        dumper: &MirBodyDumper,
        out: &mut String,
        block: BlockId,
        indent: usize,
    ) -> fmt::Result {
        dumper.block_header(out, block, indent)
    }

    fn write_instruction(
        &mut self,
        dumper: &MirBodyDumper,
        out: &mut String,
        instruction: InstructionId,
        indent: usize,
    ) -> fmt::Result {
        dumper.instruction(out, instruction, indent)
    }
}

pub(crate) struct PlainWriter;

impl<'a, 'db> MirBodyDumper<'a, 'db> {
    pub(crate) fn new(
        db: &'db dyn crate::Db,
        definition: Definition<'db>,
        body: &'a MirBody<'db>,
    ) -> Self {
        Self {
            db,
            definition,
            body,
        }
    }

    pub(crate) fn dump_file(
        db: &'db dyn crate::Db,
        file: SourceFileKey,
    ) -> Result<String, fmt::Error> {
        let mut out = String::new();
        for definition in all_definitions_of(db, file) {
            let body = match definition {
                Definition::Function(key) => function_mir_of(db, key),
                Definition::Constant(key) => constant_mir_of(db, key),
            };
            match body {
                Some(body) => out.push_str(&MirBodyDumper::new(db, definition, body).dump()?),
                None => writeln!(
                    out,
                    "{} {} has errors",
                    keyword(definition),
                    name(db, definition)
                )?,
            }
            out.push('\n');
        }
        Ok(out)
    }

    pub(crate) fn dump(&self) -> Result<String, fmt::Error> {
        self.dump_with(&mut PlainWriter)
    }

    pub(crate) fn dump_with<FW: BodyWriter>(&self, writer: &mut FW) -> Result<String, fmt::Error> {
        let mut out = String::new();
        self.decorate_body(writer, &mut out)?;
        Ok(out)
    }

    pub(crate) fn signature_line(&self) -> String {
        let name = name(self.db, self.definition);
        let parameters = self
            .body
            .signature
            .parameter_types
            .iter()
            .map(|&ty| ty.display(self.db))
            .collect::<Vec<_>>()
            .join(", ");
        let return_type = self.body.signature.return_type.display(self.db);
        format!("{name}({parameters}) -> {return_type}")
    }

    pub(crate) fn block_header(
        &self,
        out: &mut String,
        block: BlockId,
        indent: usize,
    ) -> fmt::Result {
        write!(out, "{:1$}Block{2}", "", indent - 4, block.index())?;

        let parameters = self.body.cfg.get_block(block).parameters();
        if !parameters.is_empty() {
            let rendered = parameters
                .iter()
                .map(|&parameter| {
                    let ty = self.body.cfg.get_value(parameter).ty();
                    format!("v{}: {}", parameter.index(), ty.display(self.db))
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

        let view = self.body.cfg.get_instruction(instruction);

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
            InstructionRef::Constant { constant } => {
                let name = name(self.db, Definition::Constant(constant));
                write!(out, "Constant {name}")?;
            }
            InstructionRef::FunctionAddress { function } => {
                let name = name(self.db, Definition::Function(function));
                write!(out, "FunctionAddress {name}")?;
            }
            InstructionRef::Call {
                function,
                argument_ids,
            } => {
                let name = name(self.db, Definition::Function(function));
                write!(out, "Call {name}({})", join_values(argument_ids))?;
            }
            InstructionRef::CallIndirect {
                callee_and_argument_ids,
            } => {
                let (callee_id, argument_ids) = callee_and_argument_ids.split_first().unwrap();
                write!(
                    out,
                    "CallIndirect v{}({})",
                    callee_id.index(),
                    join_values(argument_ids)
                )?;
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

    fn decorate_body<FW: BodyWriter>(&self, writer: &mut FW, out: &mut String) -> fmt::Result {
        // spec line
        writeln!(
            out,
            "{} {} {{",
            keyword(self.definition),
            self.signature_line()
        )?;

        // Instructions indent 4; block headers sit outdented 4 from that.
        let indent = 4;

        // immediate-target → values aliased directly to it
        let mut aliases = self.alias_map();

        // Iterate the layout, not the dfg — block order is the layout's
        // business.
        let mut first = true;
        for block in self.body.cfg.block_ids() {
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

    fn decorate_block<FW: BodyWriter>(
        &self,
        writer: &mut FW,
        out: &mut String,
        aliases: &mut HashMap<ValueId, Vec<ValueId>>,
        block: BlockId,
        indent: usize,
    ) -> fmt::Result {
        writer.write_block_header(self, out, block, indent)?;

        for &parameter in self.body.cfg.get_block(block).parameters() {
            self.write_value_aliases(out, aliases, parameter, indent)?;
        }

        for instruction in self.body.cfg.get_block(block).instructions() {
            writer.write_instruction(self, out, instruction, indent)?;
            for &result in self.body.cfg.get_instruction(instruction).results() {
                self.write_value_aliases(out, aliases, result, indent)?;
            }
        }
        Ok(())
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
        for value in self.body.cfg.ssa_ids() {
            if let Some(target) = self.body.cfg.get_value(value).alias_target() {
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

impl BodyWriter for PlainWriter {}

fn keyword(definition: Definition<'_>) -> &'static str {
    match definition {
        Definition::Function(_) => "function",
        Definition::Constant(_) => "constant",
    }
}

fn name<'db>(db: &'db dyn crate::Db, definition: Definition<'db>) -> &'db str {
    match definition {
        Definition::Function(key) => function_signature_of(db, key).name.text(db),
        Definition::Constant(key) => constant_signature_of(db, key).name.text(db),
    }
}

fn join_values(values: &[ValueId]) -> String {
    values
        .iter()
        .map(|value| format!("v{}", value.index()))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::core::db::BlueberryDatabase;

    #[test]
    fn test_mir_dump() {
        insta::glob!("snapshot_inputs", "**/*.bb", |path| {
            let input = fs::read_to_string(path).unwrap();
            let db = BlueberryDatabase::default();
            let file = SourceFileKey::new(&db, path.to_path_buf(), input);
            insta::assert_snapshot!(MirBodyDumper::dump_file(&db, file).unwrap());
        })
    }
}
