use crate::ast::{Expression, LogicalExpression, Predicate};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct Lir {
    pub codes: Vec<LirCode>,
}

impl Lir {
    pub fn new() -> Self {
        Self { codes: Vec::new() }
    }
}

impl Default for Lir {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub enum LirCode {
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
        translate_helper(&self, &mut lir);
        lir
    }
}

fn translate_helper(exp: &Expression, lir: &mut Lir) {
    match exp {
        Expression::Logical(logic_exp) => match logic_exp.as_ref() {
            LogicalExpression::And(l, r) => {
                translate_helper(l, lir);
                translate_helper(r, lir);
                lir.codes
                    .push(LirCode::LogicalOperator(LirLogicalOperators::And));
            }
            LogicalExpression::Or(l, r) => {
                translate_helper(l, lir);
                translate_helper(r, lir);
                lir.codes
                    .push(LirCode::LogicalOperator(LirLogicalOperators::Or));
            }
            LogicalExpression::Not(r) => {
                translate_helper(r, lir);
                lir.codes
                    .push(LirCode::LogicalOperator(LirLogicalOperators::Not));
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

            lir.codes.push(LirCode::Predicate(predicate));
        }
    }
}
