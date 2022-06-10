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
                let lhs_value = match context.value_of(&p.lhs.var_name) {
                    None => return false,
                    Some(v) => v,
                };

                match p.op {
                    BinaryOperator::Equals => {
                        if lhs_value == &p.rhs {
                            m.matches.insert(p.lhs.var_name.clone(), p.rhs.clone());
                            return true;
                        }

                        false
                    }
                    BinaryOperator::NotEquals => lhs_value != &p.rhs,
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
                        if reg_cap.is_none() {
                            return false;
                        }

                        let reg_cap = reg_cap.unwrap();
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
                        for n in rhs.capture_names() {
                            if let Some(n) = n {
                                if let Some(value) = reg_cap.name(n) {
                                    m.captures.insert(n.to_string(), value.as_str().to_string());
                                }
                            }
                        }

                        true
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

                        false
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

                        false
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

                        lhs > rhs
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

                        lhs >= rhs
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

                        lhs < rhs
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

                        lhs <= rhs
                    }
                    BinaryOperator::In => match (lhs_value, &p.rhs) {
                        (Value::String(l), Value::String(r)) => r.contains(l),
                        (Value::IpAddr(l), Value::IpCidr(r)) => r.contains(l),
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

                        !rhs.contains(lhs)
                    }
                }
            }
        }
    }
}
