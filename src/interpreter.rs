use crate::ast::{BinaryOperator, Expression, LogicalExpression, Predicate, Value};
use crate::context::{Context, Match};
use crate::lir::{Lir, LirCode, LirLogicalOperators};

pub trait Execute {
    fn execute(&self, ctx: &mut Context, m: &mut Match) -> bool;
}

impl Execute for Expression {
    fn execute(&self, ctx: &mut Context, m: &mut Match) -> bool {
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

#[derive(Debug, Clone, Copy)]
pub enum OperandItem<'a> {
    Val(bool),
    Predicate(&'a Predicate),
}
fn evaluate_operand_item(item: OperandItem, ctx: &mut Context, m: &mut Match) -> bool {
    match item {
        OperandItem::Val(b) => b,
        OperandItem::Predicate(p) => p.execute(ctx, m),
    }
}

impl Execute for Lir {
    fn execute(&self, ctx: &mut Context, m: &mut Match) -> bool {
        //operand stack
        let mut top: usize = 0;
        let mut operand_stack: [OperandItem; 2] = [OperandItem::Val(false); 2];

        for code in &self.codes {
            match code {
                LirCode::LogicalOperator(op) => {
                    match op {
                        LirLogicalOperators::And => {
                            let left = evaluate_operand_item(operand_stack[top - 2], ctx, m);
                            if !left {
                                // short circuit
                                // stack pop
                                top -= 2;

                                //stack push
                                operand_stack[top] = OperandItem::Val(false);
                                top += 1;
                            } else {
                                let right = evaluate_operand_item(operand_stack[top - 1], ctx, m);

                                //stack pop
                                top -= 2;
                                //stack push
                                operand_stack[top] = OperandItem::Val(right);
                                top += 1;
                            }
                        }
                        LirLogicalOperators::Or => {
                            let left = evaluate_operand_item(operand_stack[top - 2], ctx, m);
                            if left {
                                // short circuit
                                // stack pop
                                top -= 2;

                                //stack push
                                operand_stack[top] = OperandItem::Val(true);
                                top += 1;
                            } else {
                                let right = evaluate_operand_item(operand_stack[top - 1], ctx, m);

                                //stack pop
                                top -= 2;
                                //stack push
                                operand_stack[top] = OperandItem::Val(right);
                                top += 1;
                            }
                        }
                        LirLogicalOperators::Not => {
                            //stack pop
                            top -= 1;
                            let operand = evaluate_operand_item(operand_stack[top], ctx, m);

                            //stack push
                            operand_stack[top] = OperandItem::Val(!operand);
                            top += 1;
                        }
                    }
                }
                LirCode::Predicate(p) => {
                    // push stack
                    operand_stack[top] = OperandItem::Predicate(p);
                    top += 1;
                }
            }
        }
        //stack pop
        top -= 1;
        if top != 0 {
            panic!("There may be something wrong for the AST conversion!")
        }
        evaluate_operand_item(operand_stack[top], ctx, m) //stack pop
    }
}

impl Execute for Predicate {
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
                BinaryOperator::Less => {
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
                BinaryOperator::LessOrEqual => {
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
                BinaryOperator::NotIn => match (lhs_value, &self.rhs) {
                    (Value::IpAddr(l), Value::IpCidr(r)) => {
                        if !r.contains(l) {
                            matched = true;
                            if any {
                                return true;
                            }
                        }
                    }
                    _ => unreachable!(),
                },
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

    assert_eq!(p.execute(&mut ctx, &mut mat), false);

    // check if any value matches starts_with foo -- should be false
    let p = Predicate {
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
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), true);

    // check if all values match ends_with foo -- should be false
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), false);

    // check if any value matches ends_with foo -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), true);

    // check if any value matches starts_with foo -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), true);

    // check if any value matches ends_with nar -- should be false
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("nar".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), false);

    // check if any value matches ends_with empty string -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), true);

    // check if any value matches starts_with empty string -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), true);

    // check if any value matches contains `ob` -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("ob".to_string()),
        op: BinaryOperator::Contains,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), true);

    // check if any value matches contains `ok` -- should be false
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("ok".to_string()),
        op: BinaryOperator::Contains,
    };

    assert_eq!(p.execute(&mut ctx, &mut mat), false);
}
