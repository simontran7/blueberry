use crate::core::common::context::CompilerContext;
use crate::core::semantic_analysis::hir::{
    BindingId, BindingKind, DefinitionId, DefinitionKind, ExpressionId, ExpressionKind, Hir,
    StatementId, StatementKind,
};

use std::fmt::{self, Write};

pub(crate) struct HirDumper<'a> {
    hir: &'a Hir,
    ctx: &'a CompilerContext,
}

impl<'a> HirDumper<'a> {
    const INDENT: &'static str = "  ";

    pub(crate) const fn new(hir: &'a Hir, ctx: &'a CompilerContext) -> Self {
        HirDumper { hir, ctx }
    }

    pub(crate) fn dump(&self) -> Result<String, fmt::Error> {
        let mut hir_output = String::new();
        for &definition_id in self
            .hir
            .get_definition_ids(self.hir.source_file.definition_id_span)
        {
            self.dump_definition(definition_id, 0, &mut hir_output)?;
            hir_output.push('\n');
        }
        Ok(hir_output)
    }

    fn dump_definition(
        &self,
        definition_id: DefinitionId,
        depth: usize,
        hir_output: &mut String,
    ) -> fmt::Result {
        let definition = self.hir.get_definition(definition_id);
        let padding = Self::pad(depth);

        match *definition.kind() {
            DefinitionKind::Function {
                definition_binding_id,
                parameter_id_span,
                body_id,
            } => {
                let binding_view = self.hir.get_definition_binding(definition_binding_id);
                let name = binding_view.name();
                let ty = self.ctx.type_interner.to_string(binding_view.ty());

                // dump header
                writeln!(hir_output, "{padding}func {name} : {ty}")?;

                // dump parameter
                for &local_binding_id in self.hir.get_parameter_binding_ids(parameter_id_span) {
                    let local_binding_view = self.hir.get_local_binding(local_binding_id);
                    let parameter_name = local_binding_view.name();
                    writeln!(
                        hir_output,
                        "{}  parameter {parameter_name} : {}",
                        padding,
                        self.ctx.type_interner.to_string(local_binding_view.ty())
                    )?;
                }

                // dump block
                self.dump_expression(body_id, depth + 1, "", hir_output)?;
            }
            DefinitionKind::Constant {
                definition_binding_id,
                initializer_id,
            } => {
                let binding_view = self.hir.get_definition_binding(definition_binding_id);
                let name = binding_view.name();
                let ty = self.ctx.type_interner.to_string(binding_view.ty());

                // dump header
                writeln!(hir_output, "{padding}const {name} : {ty}")?;

                // dump expression
                self.dump_expression(initializer_id, depth + 1, "", hir_output)?;
            }
        }

        Ok(())
    }

    fn dump_statement(
        &self,
        statement_id: StatementId,
        depth: usize,
        label: &str,
        hir_output: &mut String,
    ) -> fmt::Result {
        let statement = self.hir.get_statement(statement_id);
        let padding = Self::pad(depth);

        match *statement.kind() {
            StatementKind::Expression { expression_id, .. } => {
                self.dump_expression(expression_id, depth, label, hir_output)?;
            }
            StatementKind::Let {
                pattern_id,
                value_id,
            } => {
                let binding_view = self.hir.get_local_binding(pattern_id);
                let name = binding_view.name();
                let mutability = if binding_view.mutable() { "mut " } else { "" };
                let ty = self.ctx.type_interner.to_string(binding_view.ty());

                // dump the whole statement inline or with the expression nested
                match value_id {
                    None => {
                        writeln!(hir_output, "{padding}{label}let {mutability}{name} : {ty}")?;
                    }
                    Some(value_id) => {
                        if let Some(v) = self.try_inline(value_id) {
                            writeln!(
                                hir_output,
                                "{padding}{label}let {mutability}{name} : {ty} = {v}"
                            )?;
                        } else {
                            writeln!(
                                hir_output,
                                "{padding}{label}let {mutability}{name} : {ty} ="
                            )?;
                            self.dump_expression(value_id, depth + 1, "", hir_output)?;
                        }
                    }
                }
            }
            StatementKind::Definition { definition_id } => {
                self.dump_definition(definition_id, depth, hir_output)?;
            }
        }

        Ok(())
    }

    fn dump_expression(
        &self,
        expression_id: ExpressionId,
        depth: usize,
        label: &str,
        hir_output: &mut String,
    ) -> fmt::Result {
        let expression = self.hir.get_expression(expression_id);
        let padding = Self::pad(depth);
        let ty = self.ctx.type_interner.to_string(expression.ty());

        match *expression.kind() {
            ExpressionKind::Missing => {
                writeln!(hir_output, "{padding}{label}<missing> : {ty}")?;
            }
            ExpressionKind::Block {
                statement_id_span,
                tail_id,
            } => {
                // dump header
                writeln!(hir_output, "{padding}{label}Block : {ty}")?;

                // dump statement
                for &statement_id in self.hir.get_statement_ids(statement_id_span) {
                    self.dump_statement(statement_id, depth + 1, "", hir_output)?;
                }

                // dump tail (if it exists)
                if let Some(tail_id) = tail_id {
                    self.dump_expression(tail_id, depth + 1, "[tail] ", hir_output)?;
                }
            }
            ExpressionKind::Assign {
                target_id,
                value_id,
            } => {
                // dump header
                writeln!(hir_output, "{padding}{label}assign : {ty}")?;

                // dump target
                self.dump_expression(target_id, depth + 1, "[target] ", hir_output)?;

                // dump value
                self.dump_expression(value_id, depth + 1, "[value] ", hir_output)?;
            }
            ExpressionKind::Integer(v) => {
                writeln!(hir_output, "{padding}{label}{v} : {ty}")?;
            }
            ExpressionKind::Boolean(b) => {
                writeln!(hir_output, "{padding}{label}{b} : {ty}")?;
            }
            ExpressionKind::Unit => {
                writeln!(hir_output, "{padding}{label}() : {ty}")?;
            }
            ExpressionKind::Variable(binding_id) => {
                let name = self.binding_name(binding_id);
                writeln!(hir_output, "{padding}{label}{name} : {ty}")?;
            }
            ExpressionKind::Unary {
                operator,
                operand_id,
            } => {
                // dump header
                writeln!(hir_output, "{padding}{label}`{operator}` : {ty}")?;

                // dump operand
                self.dump_expression(operand_id, depth + 1, "", hir_output)?;
            }
            ExpressionKind::Binary {
                operator,
                lhs_id,
                rhs_id,
            } => {
                // dump header
                writeln!(hir_output, "{padding}{label}`{operator}` : {ty}")?;

                // dump lhs
                self.dump_expression(lhs_id, depth + 1, "", hir_output)?;

                // dump rhs
                self.dump_expression(rhs_id, depth + 1, "", hir_output)?;
            }
            ExpressionKind::Call {
                callee_id,
                argument_id_span,
            } => {
                // dump header
                writeln!(hir_output, "{padding}{label}call : {ty}")?;

                // dump callee
                self.dump_expression(callee_id, depth + 1, "[callee] ", hir_output)?;

                // dump arguments
                for &argument_id in self.hir.get_expression_ids(argument_id_span) {
                    self.dump_expression(argument_id, depth + 1, "", hir_output)?;
                }
            }
            ExpressionKind::If {
                condition_id,
                then_branch_id,
                else_branch_id,
            } => {
                // dump header
                writeln!(hir_output, "{padding}{label}if : {ty}")?;

                // dump condition
                self.dump_expression(condition_id, depth + 1, "[condition] ", hir_output)?;

                // dump then branch
                self.dump_expression(then_branch_id, depth + 1, "[then branch] ", hir_output)?;

                // dump else branch (if it exists)
                if let Some(else_branch_id) = else_branch_id {
                    self.dump_expression(else_branch_id, depth + 1, "[else branch] ", hir_output)?;
                }
            }
            ExpressionKind::Return { value_id } => {
                // dump header
                writeln!(hir_output, "{padding}{label}return : {ty}")?;

                // dump value
                if let Some(value_id) = value_id {
                    self.dump_expression(value_id, depth + 1, "", hir_output)?;
                }
            }
            ExpressionKind::Loop { body_id, source } => {
                // dump header
                writeln!(hir_output, "{padding}{label}{} : {ty}", source.keyword())?;

                // dump body
                self.dump_expression(body_id, depth + 1, "[body] ", hir_output)?;
            }
            ExpressionKind::Break { value_id } => {
                // dump header
                writeln!(hir_output, "{padding}{label}break : {ty}")?;

                // dump value
                if let Some(value_id) = value_id {
                    self.dump_expression(value_id, depth + 1, "", hir_output)?;
                }
            }
            ExpressionKind::Continue => {
                writeln!(hir_output, "{padding}{label}continue : {ty}")?;
            }
        }

        Ok(())
    }

    fn try_inline(&self, expression_id: ExpressionId) -> Option<String> {
        match *self.hir.get_expression(expression_id).kind() {
            ExpressionKind::Integer(v) => Some(v.to_string()),
            ExpressionKind::Boolean(b) => Some(b.to_string()),
            ExpressionKind::Unit => Some("()".to_string()),
            ExpressionKind::Variable(binding_id) => Some(self.binding_name(binding_id)),
            _ => None,
        }
    }

    fn binding_name(&self, binding_id: BindingId) -> String {
        if binding_id.is_error() {
            return "<error>".into();
        }
        let name = match binding_id.kind() {
            BindingKind::Local => self
                .hir
                .get_local_binding(binding_id.as_local().unwrap())
                .name(),
            BindingKind::Definition => self
                .hir
                .get_definition_binding(binding_id.as_definition().unwrap())
                .name(),
        };
        name.to_string()
    }

    fn pad(level: usize) -> String {
        Self::INDENT.repeat(level)
    }
}
