use crate::ast::{BinaryOperator, Expression, LogicalExpression, Predicate, Value};
use crate::context::{Context, Match};

pub trait Execute {
    fn execute(&self, ctx: &Context, m: &mut Match) -> bool;
}

impl Execute for Expression {
    fn execute(&self, ctx: &Context, m: &mut Match) -> bool {
        match self {
            Expression::Logical(l) => match l.as_ref() {
                LogicalExpression::And(l, r) => l.execute(ctx, m) && r.execute(ctx, m),
                LogicalExpression::Or(l, r) => l.execute(ctx, m) || r.execute(ctx, m),
                LogicalExpression::Not(r) => !r.execute(ctx, m),
            },
            Expression::Predicate(p) => p.execute(ctx, m),
        }
    }
}

macro_rules! get_op_value {
    ($value:expr, $variant:path) => {
        match $value {
            $variant(inner) => inner,
            _ => unreachable!(),
        }
    };
}

impl Execute for Predicate {
    fn execute(&self, ctx: &Context, m: &mut Match) -> bool {
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
                    let lhs = get_op_value!(lhs_value, Value::String);
                    let rhs = get_op_value!(&self.rhs, Value::Regex);

                    if rhs.is_match(lhs) {
                        let reg_cap = rhs.captures(lhs).unwrap();

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
                    let lhs = get_op_value!(lhs_value, Value::String);
                    let rhs = get_op_value!(&self.rhs, Value::String);

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
                    let lhs = get_op_value!(lhs_value, Value::String);
                    let rhs = get_op_value!(&self.rhs, Value::String);

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
                    let lhs = get_op_value!(lhs_value, Value::Int);
                    let rhs = get_op_value!(&self.rhs, Value::Int);

                    if lhs > rhs {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::GreaterOrEqual => {
                    let lhs = get_op_value!(lhs_value, Value::Int);
                    let rhs = get_op_value!(&self.rhs, Value::Int);

                    if lhs >= rhs {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::Less => {
                    let lhs = get_op_value!(lhs_value, Value::Int);
                    let rhs = get_op_value!(&self.rhs, Value::Int);

                    if lhs < rhs {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::LessOrEqual => {
                    let lhs = get_op_value!(lhs_value, Value::Int);
                    let rhs = get_op_value!(&self.rhs, Value::Int);

                    if lhs <= rhs {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::In => {
                    let lhs = get_op_value!(lhs_value, Value::IpAddr);
                    let rhs = get_op_value!(&self.rhs, Value::IpCidr);

                    if rhs.contains(lhs) {
                      matched = true;
                      if any {
                        return true;
                      }
                    }
                },
                BinaryOperator::NotIn => {
                    let lhs = get_op_value!(lhs_value, Value::IpAddr);
                    let rhs = get_op_value!(&self.rhs, Value::IpCidr);

                      if !rhs.contains(lhs) {
                          matched = true;
                          if any {
                              return true;
                          }
                      }
                },
                BinaryOperator::Contains => {
                    let lhs = get_op_value!(lhs_value, Value::String);
                    let rhs = get_op_value!(&self.rhs, Value::String);

                    if lhs.contains(rhs) {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
            } // match

            if !any && !matched {
                // all and nothing matched
                return false;
            }
        } // for iter

        // if we reached here, it means that `any` did not find a match,
        // or we passed all matches for `all`. So we simply need to return
        // !any && lhs_values.len() > 0 to cover both cases
        !any && !lhs_values.is_empty()
    }
}

#[test]
fn test_predicate() {
    use crate::ast;
    use crate::schema;

    let mut mat = Match::new();
    let mut schema = schema::Schema::default();
    schema.add_field("my_key", ast::Type::String);
    let mut ctx = Context::new(&schema);

    // check when value list is empty
    // check if all values match starts_with foo -- should be false
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert!(!p.execute(&mut ctx, &mut mat));

    // check if any value matches starts_with foo -- should be false
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert!(!p.execute(&mut ctx, &mut mat));

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
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert!(p.execute(&mut ctx, &mut mat));

    // check if all values match ends_with foo -- should be false
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert!(!p.execute(&mut ctx, &mut mat));

    // check if any value matches ends_with foo -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert!(p.execute(&mut ctx, &mut mat));

    // check if any value matches starts_with foo -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert!(p.execute(&mut ctx, &mut mat));

    // check if any value matches ends_with nar -- should be false
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("nar".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert!(!p.execute(&mut ctx, &mut mat));

    // check if any value matches ends_with empty string -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert!(p.execute(&mut ctx, &mut mat));

    // check if any value matches starts_with empty string -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert!(p.execute(&mut ctx, &mut mat));

    // check if any value matches contains `ob` -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("ob".to_string()),
        op: BinaryOperator::Contains,
    };

    assert!(p.execute(&mut ctx, &mut mat));

    // check if any value matches contains `ok` -- should be false
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("ok".to_string()),
        op: BinaryOperator::Contains,
    };

    assert!(!p.execute(&mut ctx, &mut mat));
}
