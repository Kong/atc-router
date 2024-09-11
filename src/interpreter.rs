use crate::ast::{BinaryOperator, Predicate, Value};
use crate::context::{Context, Match};
use crate::lir::{is_operator, LirInstruction, LirLogicalOperators, LirProgram};

pub trait Execute {
    fn execute(&self, ctx: &mut Context, m: &mut Match) -> bool;
}

#[derive(Debug, Clone, Copy)]
pub enum OperandItem<'a> {
    Val(bool),
    Predicate(&'a Predicate),
}

#[inline]
fn evaluate_operand_item(item: OperandItem, ctx: &mut Context, m: &mut Match) -> bool {
    match item {
        OperandItem::Val(b) => b,
        OperandItem::Predicate(p) => p.execute(ctx, m),
    }
}

#[inline]
fn check_short_circuit(
    operator_stack: &Vec<LirLogicalOperators>,
    operand_stack: &Vec<bool>,
) -> bool {
    // if it could be short-circuited, return true
    if (operand_stack.len() > 0) && (operator_stack.len() > 0) {
        match &operator_stack.last().unwrap() {
            LirLogicalOperators::And => {
                let operand = operand_stack.last().unwrap();
                if *operand {
                    return false;
                } else {
                    return true;
                }
            }
            LirLogicalOperators::Or => {
                let operand = operand_stack.last().unwrap();
                if *operand {
                    return true;
                } else {
                    return false;
                }
            }
            LirLogicalOperators::Not => {
                return false;
            }
        }
    } else {
        return false;
    }
}

#[inline]
fn compact_operation_stack(
    operator_stack: &mut Vec<LirLogicalOperators>,
    operand_stack: &mut Vec<bool>,
) {
    loop {
        if operator_stack.len() > 0 {
            match &operator_stack.last().unwrap() {
                LirLogicalOperators::And => {
                    if operand_stack.len() >= 2 {
                        let right = operand_stack.pop().unwrap();
                        let left = operand_stack.pop().unwrap();
                        operand_stack.push(left && right);
                        operator_stack.pop();
                    } else {
                        break;
                    }
                }
                LirLogicalOperators::Or => {
                    if operand_stack.len() >= 2 {
                        let right = operand_stack.pop().unwrap();
                        let left = operand_stack.pop().unwrap();
                        operand_stack.push(left || right);
                        operator_stack.pop();
                    } else {
                        break;
                    }
                }
                LirLogicalOperators::Not => {
                    if operand_stack.len() >= 1 {
                        let right = operand_stack.pop().unwrap();
                        operand_stack.push(!right);
                        operator_stack.pop();
                    } else {
                        break;
                    }
                }
            }
        } else {
            // operator stack drained
            break;
        }
    }
}

impl Execute for LirProgram {
    fn execute(&self, ctx: &mut Context, m: &mut Match) -> bool {
        let mut operand_stack: Vec<bool> = Vec::new();
        let mut operator_stack: Vec<LirLogicalOperators> = Vec::new();
        let mut index = 0;
        loop {
            match &self.instructions[index] {
                LirInstruction::LogicalOperator(op) => match op {
                    LirLogicalOperators::And => {
                        let next_ins = &self.instructions[index + 1];
                        if is_operator(next_ins) {
                            // push LIR operator to ops stack
                            // next is operator
                            operator_stack.push(LirLogicalOperators::And);
                            index += 1;
                        } else {
                            if check_short_circuit(&operator_stack, &operand_stack) {
                                // short circuit
                                index += 3;
                                operator_stack.pop();
                            } else {
                                let left = evaluate_operand_item(
                                    OperandItem::Predicate(next_ins.as_predicate().unwrap()),
                                    ctx,
                                    m,
                                );

                                if !left {
                                    // short circuit
                                    index += 3;
                                    operand_stack.push(false);
                                } else {
                                    let next_next_ins = &self.instructions[index + 2];
                                    let right = evaluate_operand_item(
                                        OperandItem::Predicate(
                                            next_next_ins.as_predicate().unwrap(),
                                        ),
                                        ctx,
                                        m,
                                    );
                                    index += 3;
                                    operand_stack.push(right);
                                }

                                compact_operation_stack(&mut operator_stack, &mut operand_stack);
                            }
                        }
                    }
                    LirLogicalOperators::Or => {
                        let next_ins = &self.instructions[index + 1];
                        if is_operator(next_ins) {
                            // push LIR operator to ops stack
                            // next is operator
                            operator_stack.push(LirLogicalOperators::Or);
                            index += 1;
                        } else {
                            if check_short_circuit(&operator_stack, &operand_stack) {
                                // short circuit
                                index += 3;
                                operator_stack.pop();
                            } else {
                                let left = evaluate_operand_item(
                                    OperandItem::Predicate(next_ins.as_predicate().unwrap()),
                                    ctx,
                                    m,
                                );

                                if left {
                                    // short circuit
                                    index += 3;
                                    operand_stack.push(true);
                                } else {
                                    let next_next_ins = &self.instructions[index + 2];
                                    let right = evaluate_operand_item(
                                        OperandItem::Predicate(
                                            next_next_ins.as_predicate().unwrap(),
                                        ),
                                        ctx,
                                        m,
                                    );
                                    index += 3;
                                    operand_stack.push(right);
                                }

                                compact_operation_stack(&mut operator_stack, &mut operand_stack);
                            }
                        }
                    }
                    LirLogicalOperators::Not => {
                        let next_ins = &self.instructions[index + 1];
                        if is_operator(next_ins) {
                            // push LIR operator to ops stack
                            // next is operator
                            operator_stack.push(LirLogicalOperators::Not);
                            index += 1;
                        } else {
                            if check_short_circuit(&operator_stack, &operand_stack) {
                                // short circuit
                                index += 2;
                                operator_stack.pop();
                            } else {
                                let right = evaluate_operand_item(
                                    OperandItem::Predicate(next_ins.as_predicate().unwrap()),
                                    ctx,
                                    m,
                                );
                                index += 2;
                                operand_stack.push(!right);

                                compact_operation_stack(&mut operator_stack, &mut operand_stack);
                            }
                        }
                    }
                },
                LirInstruction::Predicate(p) => {
                    if check_short_circuit(&operator_stack, &operand_stack) {
                        // short circuit
                        operator_stack.pop();
                    } else {
                        let right = evaluate_operand_item(OperandItem::Predicate(p), ctx, m);
                        operand_stack.push(right);
                        compact_operation_stack(&mut operator_stack, &mut operand_stack);
                    }
                    index += 1;
                }
            }

            if index >= self.instructions.len() {
                // end of LirProgram
                break;
            }
        }
        debug_assert_eq!(operand_stack.len(), 1);
        operand_stack.pop().unwrap()
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
