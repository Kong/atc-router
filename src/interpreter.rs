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
            Expression::Predicate(p) => {
                let lhs_values = match context.value_of(&p.lhs.var_name) {
                    None => return false,
                    Some(v) => v,
                };

                let (lower, any) = p.lhs.get_transformations();

                // if not in "any" mode, then we need to check all values. 
                // `remaining` is the count of unchecked values.
                let mut remaining = lhs_values.len();
                for mut lhs_value in lhs_values
                    .iter()
                {
                    let lhs_value_transformed;

                    if lower {
                        match lhs_value {
                            Value::String(s) => {
                                lhs_value_transformed = Value::String(s.to_lowercase());
                                lhs_value = &lhs_value_transformed;
                            }
                            _ => unreachable!(),
                        }
                    }

                    match p.op {
                        BinaryOperator::Equals => {
                            if lhs_value == &p.rhs {
                                m.matches.insert(p.lhs.var_name.clone(), p.rhs.clone());
                                if any || remaining == 1{
                                    return true;
                                }
                                remaining -= 1;
                                continue;
                            }
                        }
                        BinaryOperator::NotEquals => {
                            if lhs_value != &p.rhs {
                                return true;
                            }
                        }
                        BinaryOperator::Regex => {
                            let rhs = match &p.rhs {
                                Value::Regex(r) => r,
                                _ => unreachable!(),
                            };
                            let lhs = match lhs_value {
                                Value::String(s) => s,
                                _ => unreachable!(),
                            };

                            let reg_cap = rhs.captures(lhs);
                            if let Some(reg_cap) = reg_cap {
                                m.matches.insert(
                                    p.lhs.var_name.clone(),
                                    Value::String(reg_cap.get(0).unwrap().as_str().to_string()),
                                );

                                for (i, c) in reg_cap.iter().enumerate() {
                                    if let Some(c) = c {
                                        m.captures.insert(i.to_string(), c.as_str().to_string());
                                    }
                                }

                                // named captures
                                for n in rhs.capture_names().flatten() {
                                    if let Some(value) = reg_cap.name(n) {
                                        m.captures
                                            .insert(n.to_string(), value.as_str().to_string());
                                    }
                                }

                                return true;
                            }
                        }
                        BinaryOperator::Prefix => {
                            let rhs = match &p.rhs {
                                Value::String(s) => s,
                                _ => unreachable!(),
                            };
                            let lhs = match lhs_value {
                                Value::String(s) => s,
                                _ => unreachable!(),
                            };

                            if lhs.starts_with(rhs) {
                                m.matches.insert(p.lhs.var_name.clone(), p.rhs.clone());

                                return true;
                            }
                        }
                        BinaryOperator::Postfix => {
                            let rhs = match &p.rhs {
                                Value::String(s) => s,
                                _ => unreachable!(),
                            };
                            let lhs = match lhs_value {
                                Value::String(s) => s,
                                _ => unreachable!(),
                            };

                            if lhs.ends_with(rhs) {
                                m.matches.insert(p.lhs.var_name.clone(), p.rhs.clone());

                                return true;
                            }
                        }
                        BinaryOperator::Greater => {
                            let rhs = match &p.rhs {
                                Value::Int(i) => i,
                                _ => unreachable!(),
                            };
                            let lhs = match lhs_value {
                                Value::Int(i) => i,
                                _ => unreachable!(),
                            };

                            if lhs > rhs {
                                return true;
                            }
                        }
                        BinaryOperator::GreaterOrEqual => {
                            let rhs = match &p.rhs {
                                Value::Int(i) => i,
                                _ => unreachable!(),
                            };
                            let lhs = match lhs_value {
                                Value::Int(i) => i,
                                _ => unreachable!(),
                            };

                            if lhs >= rhs {
                                return true;
                            }
                        }
                        BinaryOperator::Lesser => {
                            let rhs = match &p.rhs {
                                Value::Int(i) => i,
                                _ => unreachable!(),
                            };
                            let lhs = match lhs_value {
                                Value::Int(i) => i,
                                _ => unreachable!(),
                            };

                            if lhs < rhs {
                                return true;
                            }
                        }
                        BinaryOperator::LesserOrEqual => {
                            let rhs = match &p.rhs {
                                Value::Int(i) => i,
                                _ => unreachable!(),
                            };
                            let lhs = match lhs_value {
                                Value::Int(i) => i,
                                _ => unreachable!(),
                            };

                            if lhs <= rhs {
                                return true;
                            }
                        }
                        BinaryOperator::In => match (lhs_value, &p.rhs) {
                            (Value::String(l), Value::String(r)) => {
                                if r.contains(l) {
                                    return true;
                                }
                            }
                            (Value::IpAddr(l), Value::IpCidr(r)) => {
                                if r.contains(l) {
                                    return true;
                                }
                            }
                            _ => unreachable!(),
                        },
                        BinaryOperator::NotIn => {
                            let rhs = match &p.rhs {
                                Value::String(s) => s,
                                _ => unreachable!(),
                            };
                            let lhs = match lhs_value {
                                Value::String(s) => s,
                                _ => unreachable!(),
                            };

                            if !rhs.contains(lhs) {
                                return true;
                            }
                        }
                    }
                }

                // no match for all values
                false
            }
        }
    }
}
