use crate::ast::{self, BinaryOperator, Expression, LogicalExpression, Type, Value};
use crate::context::{Context, Match};
use crate::schema;

pub trait Execute {
    fn execute(&self, ctx: &mut Context, m: &mut Match) -> bool;
}

impl Execute for Expression {
    fn execute(&self, ctx: &mut Context, m: &mut Match) -> bool {
        match self {
            Expression::Logical(l) => match l.as_ref() {
                LogicalExpression::And(l, r) => l.execute(ctx, m) && r.execute(ctx, m),
                LogicalExpression::Or(l, r) => l.execute(ctx, m) || r.execute(ctx, m),
            },
            Expression::Predicate(p) => p.execute(ctx, m),
        }
    }
}

impl Execute for ast::Predicate {
    fn execute(&self, ctx: &mut Context, m: &mut Match) -> bool {
        let lhs_values = match ctx.value_of(&self.lhs.var_name) {
            None => return false,
            Some(v) => v,
        };

        let (lower, any) = self.lhs.get_transformations();

        // can only be "all" or "any" mode.
        // - all: all values must match (default)
        // - any: ok if any any matched
        for mut lhs_value in lhs_values.iter() {
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

            let mut matched = false;
            match self.op {
                BinaryOperator::Equals => {
                    if lhs_value == &self.rhs {
                        m.matches
                            .insert(self.lhs.var_name.clone(), self.rhs.clone());

                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::NotEquals => {
                    if lhs_value != &self.rhs {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::Regex => {
                    let rhs = match &self.rhs {
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
                            self.lhs.var_name.clone(),
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
                                m.captures.insert(n.to_string(), value.as_str().to_string());
                            }
                        }

                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::Prefix => {
                    let rhs = match &self.rhs {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };
                    let lhs = match lhs_value {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };

                    if lhs.starts_with(rhs) {
                        m.matches
                            .insert(self.lhs.var_name.clone(), self.rhs.clone());
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::Postfix => {
                    let rhs = match &self.rhs {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };
                    let lhs = match lhs_value {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };

                    if lhs.ends_with(rhs) {
                        m.matches
                            .insert(self.lhs.var_name.clone(), self.rhs.clone());
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::Greater => {
                    let rhs = match &self.rhs {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };
                    let lhs = match lhs_value {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };

                    if lhs > rhs {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::GreaterOrEqual => {
                    let rhs = match &self.rhs {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };
                    let lhs = match lhs_value {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };

                    if lhs >= rhs {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::Lesser => {
                    let rhs = match &self.rhs {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };
                    let lhs = match lhs_value {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };

                    if lhs < rhs {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::LesserOrEqual => {
                    let rhs = match &self.rhs {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };
                    let lhs = match lhs_value {
                        Value::Int(i) => i,
                        _ => unreachable!(),
                    };

                    if lhs <= rhs {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::In => match (lhs_value, &self.rhs) {
                    (Value::String(l), Value::String(r)) => {
                        if r.contains(l) {
                            matched = true;
                            if any {
                                return true;
                            }
                        }
                    }
                    (Value::IpAddr(l), Value::IpCidr(r)) => {
                        if r.contains(l) {
                            matched = true;
                            if any {
                                return true;
                            }
                        }
                    }
                    _ => unreachable!(),
                },
                BinaryOperator::NotIn => {
                    let rhs = match &self.rhs {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };
                    let lhs = match lhs_value {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };

                    if !rhs.contains(lhs) {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::Contains => {
                    let rhs = match &self.rhs {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };
                    let lhs = match lhs_value {
                        Value::String(s) => s,
                        _ => unreachable!(),
                    };

                    if lhs.contains(rhs) {
                        return true;
                    }
                }
            } // match

            if !any && !matched {
                return false;
            }
        } // for iter

        return lhs_values.len() > 0 && !any;
    }
}

#[test]
fn test_predicate() {
    let mut mat = Match::new();
    let mut schema = schema::Schema::default();
    schema.add_field("my_key", Type::String);
    let mut ctx = Context::new(&schema);

    // check when value list is empty
    // check if all values match starts_with foo -- should be false
    let p = ast::Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), false);

    // check if any value matches starts_with foo -- should be false
    let p = ast::Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), false);

    // test any mode
    let lhs_values = vec![
        Value::String("foofoo".to_string()),
        Value::String("foobar".to_string()),
        Value::String("foocar".to_string()),
        Value::String("fooban".to_string()),
    ];

    for v in lhs_values {
        ctx.add_value("my_key", v);
    }

    // check if all values match starts_with foo -- should be true
    let p = ast::Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), true);

    // check if all values match ends_with foo -- should be false
    let p = ast::Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), false);

    // check if any value matches ends_with foo -- should be true
    let p = ast::Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), true);

    // check if any value matches starts_with foo -- should be true
    let p = ast::Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), true);

    // check if any value matches ends_with nar -- should be false
    let p = ast::Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("nar".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), false);

    // check if any value matches ends_with empty string -- should be true
    let p = ast::Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), true);

    // check if any value matches starts_with empty string -- should be true
    let p = ast::Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), true);

    // check if any value matches contains `ob` -- should be true
    let p = ast::Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("ob".to_string()),
        op: BinaryOperator::Contains,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), true);

    // check if any value matches contains `ok` -- should be false
    let p = ast::Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("ok".to_string()),
        op: BinaryOperator::Contains,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), false);
}
