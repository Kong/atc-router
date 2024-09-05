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
    fn translate(&self, route: &mut Lir);
}

impl Translate for Expression {
    fn translate(&self, route: &mut Lir) {
        match self {
            Expression::Logical(logic_exp) => match logic_exp.as_ref() {
                LogicalExpression::And(l, r) => {
                    l.translate(route);
                    r.translate(route);
                    route
                        .codes
                        .push(LirCode::LogicalOperator(LirLogicalOperators::And));
                }
                LogicalExpression::Or(l, r) => {
                    l.translate(route);
                    r.translate(route);
                    route
                        .codes
                        .push(LirCode::LogicalOperator(LirLogicalOperators::Or));
                }
                LogicalExpression::Not(r) => {
                    r.translate(route);
                    route
                        .codes
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

                route.codes.push(LirCode::Predicate(predicate));
            }
        }
    }
}
