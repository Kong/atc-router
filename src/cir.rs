use crate::ast::Predicate;
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::lir::{is_operator, LirInstruction, LirLogicalOperators, LirProgram, Translate};
use crate::semantics::FieldCounter;
use std::collections::HashMap;

#[derive(Debug)]
pub struct CirProgram {
    pub(crate) instructions: Vec<CirInstruction>,
}

impl CirProgram {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
        }
    }
}

impl Default for CirProgram {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum CirInstruction {
    AndIns(AndIns),
    OrIns(OrIns),
    NotIns(NotIns),
}

#[derive(Debug)]
pub struct AndIns {
    left: CirOperand,
    right: CirOperand,
}

#[derive(Debug)]
pub struct OrIns {
    left: CirOperand,
    right: CirOperand,
}

#[derive(Debug)]
pub struct NotIns {
    right: CirOperand,
}

#[derive(Debug, Clone)]
pub enum CirOperand {
    Index(usize),
    Predicate(Predicate),
}

impl CirOperand {
    pub fn as_predicate(&self) -> Option<&Predicate> {
        match &self {
            CirOperand::Index(_index) => None, // never be here, otherwise something wrong
            CirOperand::Predicate(p) => Some(p),
        }
    }

    pub fn as_index(&self) -> Option<usize> {
        match &self {
            CirOperand::Index(index) => Some(*index),
            CirOperand::Predicate(_p) => None, // never be here, otherwise something wrong
        }
    }
}

fn is_index(cir_operand: &CirOperand) -> bool {
    match cir_operand {
        CirOperand::Index(_index) => true,
        CirOperand::Predicate(_p) => false,
    }
}

impl Translate for LirProgram {
    type Output = CirProgram;
    fn translate(&self) -> Self::Output {
        let mut cir = CirProgram::new();
        cir_translate_helper(self, &mut cir);
        #[cfg(debug_assertions)]
        {
            use std::mem;
            println!("The number of cir instructions: {}", cir.instructions.len());
            println!(
                "The size of cir program: {} bytes",
                mem::size_of::<CirProgram>()
                    + mem::size_of::<CirInstruction>() * cir.instructions.len()
            );
        }
        cir
    }
}

#[inline]
fn reduce_translation_stack(
    cir_instructions: &mut Vec<CirInstruction>,
    operator_stack: &mut Vec<LirLogicalOperators>,
    operand_stack: &mut Vec<CirOperand>,
) {
    loop {
        if !operator_stack.is_empty() {
            match &operator_stack.last().unwrap() {
                LirLogicalOperators::And => {
                    if operand_stack.len() >= 2 {
                        let right = operand_stack.pop().unwrap();
                        let left = operand_stack.pop().unwrap();
                        let and_ins = AndIns {
                            left: left.clone(),
                            right: right.clone(),
                        };
                        cir_instructions.push(CirInstruction::AndIns(and_ins));
                        operand_stack.push(CirOperand::Index(cir_instructions.len() - 1));
                        operator_stack.pop();
                    } else {
                        break;
                    }
                }
                LirLogicalOperators::Or => {
                    if operand_stack.len() >= 2 {
                        let right = operand_stack.pop().unwrap();
                        let left = operand_stack.pop().unwrap();
                        let or_ins = OrIns {
                            left: left.clone(),
                            right: right.clone(),
                        };
                        cir_instructions.push(CirInstruction::OrIns(or_ins));
                        operand_stack.push(CirOperand::Index(cir_instructions.len() - 1));
                        operator_stack.pop();
                    } else {
                        break;
                    }
                }
                LirLogicalOperators::Not => {
                    if !operand_stack.is_empty() {
                        let right = operand_stack.pop().unwrap();
                        let not_ins = NotIns {
                            right: right.clone(),
                        };
                        cir_instructions.push(CirInstruction::NotIns(not_ins));
                        operand_stack.push(CirOperand::Index(cir_instructions.len() - 1));
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

#[inline]
fn cir_translate_helper(lir: &LirProgram, cir: &mut CirProgram) {
    let mut operand_stack: Vec<CirOperand> = Vec::new();
    let mut operator_stack: Vec<LirLogicalOperators> = Vec::new();
    let mut index = 0;
    loop {
        match &lir.instructions[index] {
            LirInstruction::LogicalOperator(op) => match op {
                LirLogicalOperators::And => {
                    let next_ins = &lir.instructions[index + 1];
                    if is_operator(next_ins) {
                        // next is operator
                        operator_stack.push(LirLogicalOperators::And);
                        index += 1;
                    } else {
                        let next_next_ins = &lir.instructions[index + 2];
                        let and_ins = AndIns {
                            left: CirOperand::Predicate(next_ins.as_predicate().unwrap().clone()),
                            right: CirOperand::Predicate(
                                next_next_ins.as_predicate().unwrap().clone(),
                            ),
                        };
                        cir.instructions.push(CirInstruction::AndIns(and_ins));
                        operand_stack.push(CirOperand::Index(cir.instructions.len() - 1));
                        index += 3;

                        reduce_translation_stack(
                            &mut cir.instructions,
                            &mut operator_stack,
                            &mut operand_stack,
                        );
                    }
                }
                LirLogicalOperators::Or => {
                    let next_ins = &lir.instructions[index + 1];
                    if is_operator(next_ins) {
                        // next is operator
                        operator_stack.push(LirLogicalOperators::Or);
                        index += 1;
                    } else {
                        let next_next_ins = &lir.instructions[index + 2];
                        let or_ins = OrIns {
                            left: CirOperand::Predicate(next_ins.as_predicate().unwrap().clone()),
                            right: CirOperand::Predicate(
                                next_next_ins.as_predicate().unwrap().clone(),
                            ),
                        };
                        cir.instructions.push(CirInstruction::OrIns(or_ins));
                        operand_stack.push(CirOperand::Index(cir.instructions.len() - 1));
                        index += 3;

                        reduce_translation_stack(
                            &mut cir.instructions,
                            &mut operator_stack,
                            &mut operand_stack,
                        );
                    }
                }
                LirLogicalOperators::Not => {
                    let next_ins = &lir.instructions[index + 1];
                    if is_operator(next_ins) {
                        // push LIR operator to ops stack
                        // next is operator
                        operator_stack.push(LirLogicalOperators::Not);
                        index += 1;
                    } else {
                        next_ins.as_predicate().unwrap(); //right
                        let not_ins = NotIns {
                            right: CirOperand::Predicate(next_ins.as_predicate().unwrap().clone()),
                        };
                        cir.instructions.push(CirInstruction::NotIns(not_ins));
                        operand_stack.push(CirOperand::Index(cir.instructions.len() - 1));
                        index += 2;

                        reduce_translation_stack(
                            &mut cir.instructions,
                            &mut operator_stack,
                            &mut operand_stack,
                        );
                    }
                }
            },
            LirInstruction::Predicate(p) => {
                operand_stack.push(CirOperand::Predicate(p.clone()));
                reduce_translation_stack(
                    &mut cir.instructions,
                    &mut operator_stack,
                    &mut operand_stack,
                );

                index += 1;
            }
        }

        if index >= lir.instructions.len() {
            // end of LirProgram
            break;
        }
    }
    debug_assert_eq!(operator_stack.len(), 0);
}

#[inline]
fn execute_helper(
    cir_instructions: &[CirInstruction],
    index: usize,
    ctx: &mut Context,
    m: &mut Match,
) -> bool {
    match &cir_instructions[index] {
        CirInstruction::AndIns(and) => {
            let left_val = if is_index(&and.left) {
                execute_helper(cir_instructions, and.left.as_index().unwrap(), ctx, m)
            } else {
                and.left.as_predicate().unwrap().execute(ctx, m)
            };

            if !left_val {
                // short circuit
                false
            } else if is_index(&and.right) {
                execute_helper(cir_instructions, and.right.as_index().unwrap(), ctx, m)
            } else {
                and.right.as_predicate().unwrap().execute(ctx, m)
            }
        }
        CirInstruction::OrIns(or) => {
            let left_val = if is_index(&or.left) {
                execute_helper(cir_instructions, or.left.as_index().unwrap(), ctx, m)
            } else {
                or.left.as_predicate().unwrap().execute(ctx, m)
            };

            if left_val {
                // short circuit
                true
            } else if is_index(&or.right) {
                execute_helper(cir_instructions, or.right.as_index().unwrap(), ctx, m)
            } else {
                or.right.as_predicate().unwrap().execute(ctx, m)
            }
        }
        CirInstruction::NotIns(not) => {
            let right_val = if is_index(&not.right) {
                execute_helper(cir_instructions, not.right.as_index().unwrap(), ctx, m)
            } else {
                not.right.as_predicate().unwrap().execute(ctx, m)
            };
            !right_val
        }
    }
}

impl Execute for CirProgram {
    fn execute(&self, ctx: &mut Context, m: &mut Match) -> bool {
        execute_helper(&self.instructions, self.instructions.len() - 1, ctx, m)
    }
}

impl FieldCounter for CirProgram {
    fn add_to_counter(&self, map: &mut HashMap<String, usize>) {
        for instruction in &self.instructions {
            match &instruction {
                CirInstruction::AndIns(and) => {
                    if !is_index(&and.left) {
                        *map.entry(and.left.as_predicate().unwrap().lhs.var_name.clone())
                            .or_default() += 1;
                    }
                    if !is_index(&and.right) {
                        *map.entry(and.right.as_predicate().unwrap().lhs.var_name.clone())
                            .or_default() += 1;
                    }
                }
                CirInstruction::OrIns(or) => {
                    if !is_index(&or.left) {
                        *map.entry(or.left.as_predicate().unwrap().lhs.var_name.clone())
                            .or_default() += 1;
                    }
                    if !is_index(&or.right) {
                        *map.entry(or.right.as_predicate().unwrap().lhs.var_name.clone())
                            .or_default() += 1;
                    }
                }
                CirInstruction::NotIns(not) => {
                    if !is_index(&not.right) {
                        *map.entry(not.right.as_predicate().unwrap().lhs.var_name.clone())
                            .or_default() += 1;
                    }
                }
            }
        }
    }

    fn remove_from_counter(&self, map: &mut HashMap<String, usize>) {
        for instruction in &self.instructions {
            match &instruction {
                CirInstruction::AndIns(and) => {
                    if !is_index(&and.left) {
                        let left = and.left.as_predicate().unwrap();
                        let val = map.get_mut(&left.lhs.var_name).unwrap();
                        *val -= 1;

                        if *val == 0 {
                            assert!(map.remove(&left.lhs.var_name).is_some());
                        }
                    }

                    if !is_index(&and.right) {
                        let right = and.right.as_predicate().unwrap();
                        let val = map.get_mut(&right.lhs.var_name).unwrap();
                        *val -= 1;

                        if *val == 0 {
                            assert!(map.remove(&right.lhs.var_name).is_some());
                        }
                    }
                }
                CirInstruction::OrIns(or) => {
                    if !is_index(&or.left) {
                        let left = or.left.as_predicate().unwrap();
                        let val = map.get_mut(&left.lhs.var_name).unwrap();
                        *val -= 1;

                        if *val == 0 {
                            assert!(map.remove(&left.lhs.var_name).is_some());
                        }
                    }

                    if !is_index(&or.right) {
                        let right = or.right.as_predicate().unwrap();
                        let val = map.get_mut(&right.lhs.var_name).unwrap();
                        *val -= 1;

                        if *val == 0 {
                            assert!(map.remove(&right.lhs.var_name).is_some());
                        }
                    }
                }
                CirInstruction::NotIns(not) => {
                    if !is_index(&not.right) {
                        let right = not.right.as_predicate().unwrap();
                        let val = map.get_mut(&right.lhs.var_name).unwrap();
                        *val -= 1;

                        if *val == 0 {
                            assert!(map.remove(&right.lhs.var_name).is_some());
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    use crate::router::Router;
    use crate::schema::Schema;
    use uuid::Uuid;

    #[test]
    fn verify_translate() {
        let mut schema = Schema::default();
        schema.add_field("a", crate::ast::Type::Int);
        schema.add_field("http.path", crate::ast::Type::String);
        schema.add_field("http.version", crate::ast::Type::String);
        let mut router = Router::new(&schema);
        let uuid = Uuid::try_from("8cb2a7d0-c775-4ed9-989f-77697240ae96").unwrap();
        //router.add_matcher(0,  uuid, r#"!(( a == 2) && ( a == 9 )) || !(a == 1) || (http.path == "hello" && http.version == "1.1") || ( a == 3 && a == 4) && !(a == 5)"#).unwrap();
        router.add_matcher(0, uuid, r#"(http.path == "hello" && http.version == "1.1") || !(( a == 2) && ( a == 9 )) || !(a == 1) || ( a == 3 && a == 4) && !(a == 5)"#).unwrap();

        let mut context = crate::context::Context::new(&schema);
        context.add_value("http.path", crate::ast::Value::String("hello".to_string()));
        context.add_value("http.version", crate::ast::Value::String("1.1".to_string()));
        assert!(router.execute(&mut context));
        println!("{:?}", context.result.unwrap().matches);
    }
}
