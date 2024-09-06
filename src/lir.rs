use crate::ast::{Expression, LogicalExpression, Predicate};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct Lir {
    pub program: Vec<LirInstruction>,
}

impl Lir {
    pub fn new() -> Self {
        Self {
            program: Vec::new(),
        }
    }
}

impl Default for Lir {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub enum LirInstruction {
    LogicalOperator(LirLogicalOperators),
    Predicate(Predicate),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub enum LirLogicalOperators {
    And,
    Or,
    Not,
}

pub trait Translate {
    type Output;
    fn translate(&self) -> Self::Output;
}

impl Translate for Expression {
    type Output = Lir;
    fn translate(&self) -> Self::Output {
        let mut lir = Lir::new();
        translate_helper(self, &mut lir);
        lir
    }
}

fn translate_helper(exp: &Expression, lir: &mut Lir) {
    match exp {
        Expression::Logical(logic_exp) => match logic_exp.as_ref() {
            LogicalExpression::And(l, r) => {
                translate_helper(l, lir);
                translate_helper(r, lir);
                lir.program
                    .push(LirInstruction::LogicalOperator(LirLogicalOperators::And));
            }
            LogicalExpression::Or(l, r) => {
                translate_helper(l, lir);
                translate_helper(r, lir);
                lir.program
                    .push(LirInstruction::LogicalOperator(LirLogicalOperators::Or));
            }
            LogicalExpression::Not(r) => {
                translate_helper(r, lir);
                lir.program
                    .push(LirInstruction::LogicalOperator(LirLogicalOperators::Not));
            }
        },
        Expression::Predicate(p) => {
            let predicate = Predicate {
                lhs: crate::ast::Lhs {
                    var_name: p.lhs.var_name.clone(),
                    transformations: p.lhs.transformations.clone(),
                },
                rhs: p.rhs.clone(),
                op: p.op,
            };

            lir.program.push(LirInstruction::Predicate(predicate));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::schema::Schema;
    use crate::semantics::Validate;

    fn format(lir: &Lir) -> String {
        let mut predicate_vec: Vec<String> = Vec::new();
        for instruction in &lir.program {
            match instruction {
                LirInstruction::LogicalOperator(op) => match op {
                    LirLogicalOperators::And => {
                        let right = predicate_vec.pop().unwrap();
                        let left = predicate_vec.pop().unwrap();
                        predicate_vec.push(format!("{} && {}", left, right));
                    }
                    LirLogicalOperators::Or => {
                        let right = predicate_vec.pop().unwrap();
                        let left = predicate_vec.pop().unwrap();
                        predicate_vec.push(format!("{} || {}", left, right));
                    }
                    LirLogicalOperators::Not => {
                        let operand = predicate_vec.pop().unwrap();
                        predicate_vec.push(format!("!({})", operand));
                    }
                },
                LirInstruction::Predicate(p) => {
                    predicate_vec.push(format!(
                        "{} {} {}",
                        p.lhs.var_name.to_string(),
                        p.op.to_string(),
                        &p.rhs.to_string()
                    ));
                }
            }
        }
        predicate_vec.pop().unwrap()
    }

    #[test]
    fn verify_translate() {
        let mut schema = Schema::default();
        schema.add_field("a", crate::ast::Type::Int);
        let test_input: &str = r#"!(!(a == 1 && a == 2) || a == 3 && !(a == 4))"#;
        let ast = parse(test_input).map_err(|e| e.to_string()).unwrap();
        ast.validate(&schema).unwrap();
        let lir = ast.translate();
        let test_result = format(&lir);
        assert_eq!(test_input, test_result, "Responses should be equal");
        //use "cargo test -- --nocapture" to show following output
        println!(" input: {}", test_input);
        println!("output: {}", test_result);
    }
}
