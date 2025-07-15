// cir:  compact intermediate representation
use crate::ast::{Expression, LogicalExpression, Predicate};
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::semantics::{FieldCounter, ValidationHashMap};

#[derive(Debug)]
pub enum CirProgram {
    Instructions(Box<[CirInstruction]>),
    Predicate(Predicate),
}

impl Default for CirProgram {
    fn default() -> Self {
        Self::Instructions(Box::new([]))
    }
}

#[derive(Debug)]
pub enum CirInstruction {
    And(CirOperand, CirOperand),
    Or(CirOperand, CirOperand),
    Not(CirOperand),
    Predicate(Predicate),
}

#[derive(Debug, Clone)]
pub enum CirOperand {
    Index(usize),
    Predicate(Predicate),
}

pub trait Translate {
    type Output;
    fn translate(&self) -> Self::Output;
}

impl Translate for Expression {
    type Output = CirProgram;

    fn translate(&self) -> Self::Output {
        let mut instructions = Vec::new();
        let len = cir_translate_helper(self, &mut instructions);

        if len > 0 {
            return CirProgram::Instructions(instructions.into_boxed_slice());
        }

        // Avoid unnecessary cloning
        match &instructions[0] {
            // Unwrap is safe here because we know that there is one and only one instruction
            CirInstruction::Predicate(_) => match instructions.pop().unwrap() {
                CirInstruction::Predicate(p) => CirProgram::Predicate(p),
                _ => unreachable!(),
            },
            _ => CirProgram::Instructions(instructions.into_boxed_slice()),
        }
    }
}

fn cir_translate_to_operand(exp: &Expression, cir: &mut Vec<CirInstruction>) -> CirOperand {
    use Expression::{Logical, Predicate};

    match exp {
        Logical(_) => CirOperand::Index(cir_translate_helper(exp, cir)),
        Predicate(p) => CirOperand::Predicate(p.clone()),
    }
}

/// Helper function for translation from AST to CIR.
/// Parameters:
///   * reference to AST
///   * reference to translated CIR
///     This function returns:
///   * index of translated IR
fn cir_translate_helper(exp: &Expression, cir: &mut Vec<CirInstruction>) -> usize {
    use Expression::{Logical, Predicate};
    use LogicalExpression::{And, Not, Or};

    match exp {
        Logical(logic_exp) => match logic_exp.as_ref() {
            And(l, r) => {
                let left = cir_translate_to_operand(l, cir);
                let right = cir_translate_to_operand(r, cir);

                cir.push(CirInstruction::And(left, right));
            }
            Or(l, r) => {
                let left = cir_translate_to_operand(l, cir);
                let right = cir_translate_to_operand(r, cir);

                cir.push(CirInstruction::Or(left, right));
            }
            Not(r) => {
                let right = cir_translate_to_operand(r, cir);

                cir.push(CirInstruction::Not(right));
            }
        },
        Predicate(p) => {
            cir.push(CirInstruction::Predicate(p.clone()));
        }
    }

    cir.len() - 1
}

fn operand_execute_helper(
    op: &CirOperand,
    instructions: &[CirInstruction],
    ctx: &Context,
    m: &mut Match,
) -> bool {
    match op {
        CirOperand::Index(index) => execute_helper(instructions, *index, ctx, m),
        CirOperand::Predicate(p) => p.execute(ctx, m),
    }
}

fn execute_helper(
    instructions: &[CirInstruction],
    index: usize,
    ctx: &Context,
    m: &mut Match,
) -> bool {
    match &instructions[index] {
        CirInstruction::And(left, right) => {
            operand_execute_helper(left, instructions, ctx, m) &&
            operand_execute_helper(right, instructions, ctx, m)
        }
        CirInstruction::Or(left, right) => {
            operand_execute_helper(left, instructions, ctx, m) ||
            operand_execute_helper(right, instructions, ctx, m)
        }
        CirInstruction::Not(right) => {
            !operand_execute_helper(right, instructions, ctx, m)
        }
        CirInstruction::Predicate(p) => p.execute(ctx, m),
    }
}

impl Execute for CirProgram {
    fn execute(&self, ctx: &Context, m: &mut Match) -> bool {
        match self {
            CirProgram::Instructions(instructions) => {
                execute_helper(instructions, instructions.len() - 1, ctx, m)
            }
            CirProgram::Predicate(p) => p.execute(ctx, m),
        }
    }
}

impl FieldCounter for CirOperand {
    fn add_to_counter(&self, map: &mut ValidationHashMap) {
        if let CirOperand::Predicate(p) = &self {
            p.add_to_counter(map);
        }
    }
    fn remove_from_counter(&self, map: &mut ValidationHashMap) {
        if let CirOperand::Predicate(p) = &self {
            p.remove_from_counter(map);
        }
    }
}

impl FieldCounter for CirInstruction {
    fn add_to_counter(&self, map: &mut ValidationHashMap) {
        match self {
            CirInstruction::And(left, right) | CirInstruction::Or(left, right) => {
                left.add_to_counter(map);
                right.add_to_counter(map);
            }
            CirInstruction::Not(right) => {
                right.add_to_counter(map);
            }
            CirInstruction::Predicate(p) => {
                p.add_to_counter(map);
            }
        }
    }
    fn remove_from_counter(&self, map: &mut ValidationHashMap) {
        match self {
            CirInstruction::And(left, right) | CirInstruction::Or(left, right) => {
                left.remove_from_counter(map);
                right.remove_from_counter(map);
            }
            CirInstruction::Not(right) => {
                right.remove_from_counter(map);
            }
            CirInstruction::Predicate(p) => {
                p.remove_from_counter(map);
            }
        }
    }
}

impl FieldCounter for Predicate {
    fn add_to_counter(&self, map: &mut ValidationHashMap) {
        *map.entry(self.lhs.var_name.clone()).or_default() += 1;
    }

    fn remove_from_counter(&self, map: &mut ValidationHashMap) {
        let val = map.get_mut(&self.lhs.var_name).unwrap();
        *val -= 1;

        if *val == 0 {
            assert!(map.remove(&self.lhs.var_name).is_some());
        }
    }
}

impl FieldCounter for CirProgram {
    fn add_to_counter(&self, map: &mut ValidationHashMap) {
        match self {
            CirProgram::Instructions(instructions) => {
                instructions
                    .iter()
                    .for_each(|instruction: &CirInstruction| instruction.add_to_counter(map));
            }
            CirProgram::Predicate(p) => p.add_to_counter(map),
        }
    }

    fn remove_from_counter(&self, map: &mut ValidationHashMap) {
        match self {
            CirProgram::Instructions(instructions) => {
                instructions
                    .iter()
                    .for_each(|instruction: &CirInstruction| instruction.remove_from_counter(map));
            }
            CirProgram::Predicate(p) => p.remove_from_counter(map),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Type, Value};
    use crate::schema::Schema;

    #[test]
    fn verify_translate_execute() {
        let mut schema = Schema::default();
        schema.add_field("a", Type::Int);
        schema.add_field("http.path", Type::String);
        schema.add_field("http.version", Type::String);

        let exprs = vec![
            r#"a == 5 "#,
            r#"!(!(a == 1 && a == 2) || a == 3 && !(a == 4))"#,
            r#"!(( a == 2) && ( a == 9 )) || !(a == 1) || (http.path == "hello" && http.version == "1.1") || ( a == 3 && a == 4) && !(a == 5)"#,
            r#"(http.path == "hello" && http.version == "1.1") || !(( a == 2) && ( a == 9 )) || !(a == 1) || ( a == 5 && a == 4) && !(a == 3)"#,
            r#"(http.path == "hello" && http.version == "1.1") || ( a == 3 && a == 4) && !(a == 5)"#,
            r#"http.path == "hello" && http.version == "1.1""#,
        ];

        let mut context = Context::new(&schema);
        context.add_value("http.path", Value::String("hello".to_string()));
        context.add_value("http.version", Value::String("1.1".to_string()));
        context.add_value("a", Value::Int(3_i64));

        for expr in exprs {
            let ast = crate::parser::parse(expr)
                .map_err(|e| e.to_string())
                .unwrap();

            let mut mat = Match::new();
            let ast_result = ast.execute(&context, &mut mat);

            let cir_result = ast.translate().execute(&context, &mut mat);
            assert_eq!(ast_result, cir_result);
        }
    }
}
