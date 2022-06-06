use crate::ast::{BinaryOperator, Expression, LogicalExpression, Value};
use crate::context::{Context, Match};

pub trait Execute {
    fn execute(&self, context: &mut Context, m: &mut Match) -> bool;
}

impl Execute for Expression {
    fn execute(&self, context: &mut Context, m: &mut Match) -> bool {
        match self {
            Expression::Logical(l) => match l.as_ref() {
                LogicalExpression::And(l, r) => l.execute(context, m) && r.execute(context, m),
                LogicalExpression::Or(l, r) => l.execute(context, m) || r.execute(context, m),
            },
            Expression::Predicate(p) => match p.op {
                BinaryOperator::Equals => context.value_of(&p.lhs.var_name) == &p.rhs,
                BinaryOperator::NotEquals => context.value_of(&p.lhs.var_name) != &p.rhs,
                BinaryOperator::Regex => {
                    let rhs = match &p.rhs {
                        Value::Regex(r) => r,
                        _ => unreachable!(),
                    };
                    let lhs = match context.value_of(&p.lhs.var_name) {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };
                    rhs.is_match(lhs)
                }
                BinaryOperator::Prefix => {
                    let rhs = match &p.rhs {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };
                    let lhs = match context.value_of(&p.lhs.var_name) {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };

                    if lhs.starts_with(rhs) {
                        if p.lhs.var_name == "http.path" {
                            // hack: prefix extraction
                            m.prefix = Some(rhs.to_string());
                        }

                        return true;
                    }

                    false
                }
                BinaryOperator::Postfix => {
                    let rhs = match &p.rhs {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };
                    let lhs = match context.value_of(&p.lhs.var_name) {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };

                    lhs.ends_with(rhs)
                }
                BinaryOperator::Greater => {
                    let rhs = match &p.rhs {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };
                    let lhs = match context.value_of(&p.lhs.var_name) {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };

                    lhs > rhs
                }
                BinaryOperator::GreaterOrEqual => {
                    let rhs = match &p.rhs {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };
                    let lhs = match context.value_of(&p.lhs.var_name) {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };

                    lhs >= rhs
                }
                BinaryOperator::Lesser => {
                    let rhs = match &p.rhs {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };
                    let lhs = match context.value_of(&p.lhs.var_name) {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };

                    lhs < rhs
                }
                BinaryOperator::LesserOrEqual => {
                    let rhs = match &p.rhs {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };
                    let lhs = match context.value_of(&p.lhs.var_name) {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };

                    lhs <= rhs
                }
                BinaryOperator::In => {
                    let rhs = match &p.rhs {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };
                    let lhs = match context.value_of(&p.lhs.var_name) {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };

                    rhs.contains(lhs)
                }
                BinaryOperator::NotIn => {
                    let rhs = match &p.rhs {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };
                    let lhs = match context.value_of(&p.lhs.var_name) {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };

                    !rhs.contains(lhs)
                }
            },
        }
    }
}
