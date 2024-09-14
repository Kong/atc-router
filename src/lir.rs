use crate::ast::{Expression, LogicalExpression, Predicate};

#[derive(Debug)]
pub struct LirProgram {
    pub(crate) instructions: Vec<LirInstruction>,
}

impl LirProgram {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
        }
    }
}

impl Default for LirProgram {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum LirInstruction {
    LogicalOperator(LirLogicalOperators),
    Predicate(Predicate),
}

impl LirInstruction {
    pub fn as_predicate(&self) -> &Predicate {
        match &self {
            LirInstruction::LogicalOperator(_ops) => {
                panic!("Call as_predicate on LogicalOperator Operand, LirProgram is wrong.")
            }
            LirInstruction::Predicate(p) => p,
        }
    }
}

#[inline]
pub(crate) fn is_operator(instruction: &LirInstruction) -> bool {
    match instruction {
        LirInstruction::LogicalOperator(_op) => true,
        LirInstruction::Predicate(_p) => false,
    }
}

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

#[cfg(debug_assertions)]
trait CountSize {
    type Output;
    fn count_size(&self) -> Self::Output;
}
#[cfg(debug_assertions)]
#[derive(Debug, Default)]
struct ExpressionInsBytes {
    ins_number: usize,
    ins_bytes: usize,
}
#[cfg(debug_assertions)]
impl ExpressionInsBytes {
    pub fn new() -> Self {
        Self {
            ins_number: 0,
            ins_bytes: 0,
        }
    }
}
#[cfg(debug_assertions)]
fn expression_count_heler(exp: &Expression, counter: &mut ExpressionInsBytes) {
    use crate::ast::{Expression, LogicalExpression};
    use std::mem;
    counter.ins_bytes += mem::size_of::<Expression>();
    match exp {
        Expression::Logical(l) => {
            counter.ins_number += 1;
            counter.ins_bytes += mem::size_of::<LogicalExpression>();
            match l.as_ref() {
                LogicalExpression::And(l, r) => {
                    expression_count_heler(l, counter);
                    expression_count_heler(r, counter);
                }
                LogicalExpression::Or(l, r) => {
                    expression_count_heler(l, counter);
                    expression_count_heler(r, counter);
                }
                LogicalExpression::Not(r) => {
                    expression_count_heler(r, counter);
                }
            }
        }
        Expression::Predicate(_p) => {}
    }
}
#[cfg(debug_assertions)]
impl CountSize for Expression {
    type Output = ExpressionInsBytes;
    fn count_size(&self) -> Self::Output {
        let mut counter = ExpressionInsBytes::new();
        expression_count_heler(self, &mut counter);
        counter
    }
}

impl Translate for Expression {
    type Output = LirProgram;
    fn translate(&self) -> Self::Output {
        let mut lir = LirProgram::new();
        lir_translate_helper(self, &mut lir);
        #[cfg(debug_assertions)]
        {
            use std::mem;
            let ast_counter = self.count_size();

            println!(
                "The size of AST instructions: {} bytes",
                ast_counter.ins_bytes
            );
            println!("The number of AST instructions: {}", ast_counter.ins_number);
            println!("The AST instructions:");
            println!("{:?}", self);

            println!(
                "The size of lir program: {} bytes",
                mem::size_of::<LirProgram>()
                    + mem::size_of::<LirInstruction>() * lir.instructions.len()
            );
            println!("The number of lir instructions: {}", lir.instructions.len());
            println!("The lir instructions:");
            println!("{:?}", lir.instructions);
        }
        lir
    }
}

fn lir_translate_helper(exp: &Expression, lir: &mut LirProgram) {
    match exp {
        Expression::Logical(logic_exp) => match logic_exp.as_ref() {
            LogicalExpression::And(l, r) => {
                lir.instructions
                    .push(LirInstruction::LogicalOperator(LirLogicalOperators::And));
                lir_translate_helper(l, lir);
                lir_translate_helper(r, lir);
            }
            LogicalExpression::Or(l, r) => {
                lir.instructions
                    .push(LirInstruction::LogicalOperator(LirLogicalOperators::Or));
                lir_translate_helper(l, lir);
                lir_translate_helper(r, lir);
            }
            LogicalExpression::Not(r) => {
                lir.instructions
                    .push(LirInstruction::LogicalOperator(LirLogicalOperators::Not));
                lir_translate_helper(r, lir);
            }
        },
        Expression::Predicate(p) => {
            lir.instructions.push(LirInstruction::Predicate(p.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{Context, Match};
    use crate::interpreter::Execute;
    use crate::router::Router;
    use crate::schema::Schema;
    use crate::semantics::FieldCounter;
    use std::collections::HashMap;
    use uuid::Uuid;

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
    fn check_short_circuit(operator_stack: &[LirLogicalOperators], operand_stack: &[bool]) -> bool {
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
                    LirInstruction::LogicalOperator(op) => {
                        let next_ins = &self.instructions[index + 1];
                        if is_operator(next_ins) {
                            // push LIR operator to ops stack
                            // next is operator
                            match op {
                                LirLogicalOperators::And => {
                                    operator_stack.push(LirLogicalOperators::And)
                                }
                                LirLogicalOperators::Or => {
                                    operator_stack.push(LirLogicalOperators::Or)
                                }
                                LirLogicalOperators::Not => {
                                    operator_stack.push(LirLogicalOperators::Not)
                                }
                            }
                            index += 1;
                        } else {
                            match op {
                                LirLogicalOperators::And => {
                                    if check_short_circuit(&operator_stack, &operand_stack) {
                                        // short circuit
                                        index += 3;
                                        operator_stack.pop();
                                    } else {
                                        let left = evaluate_operand_item(
                                            OperandItem::Predicate(next_ins.as_predicate()),
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
                                                    next_next_ins.as_predicate(),
                                                ),
                                                ctx,
                                                m,
                                            );
                                            index += 3;
                                            operand_stack.push(right);
                                        }

                                        compact_operation_stack(
                                            &mut operator_stack,
                                            &mut operand_stack,
                                        );
                                    }
                                }
                                LirLogicalOperators::Or => {
                                    if check_short_circuit(&operator_stack, &operand_stack) {
                                        // short circuit
                                        index += 3;
                                        operator_stack.pop();
                                    } else {
                                        let left = evaluate_operand_item(
                                            OperandItem::Predicate(next_ins.as_predicate()),
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
                                                    next_next_ins.as_predicate(),
                                                ),
                                                ctx,
                                                m,
                                            );
                                            index += 3;
                                            operand_stack.push(right);
                                        }

                                        compact_operation_stack(
                                            &mut operator_stack,
                                            &mut operand_stack,
                                        );
                                    }
                                }
                                LirLogicalOperators::Not => {
                                    if check_short_circuit(&operator_stack, &operand_stack) {
                                        // short circuit
                                        index += 2;
                                        operator_stack.pop();
                                    } else {
                                        let right = evaluate_operand_item(
                                            OperandItem::Predicate(next_ins.as_predicate()),
                                            ctx,
                                            m,
                                        );
                                        index += 2;
                                        operand_stack.push(!right);

                                        compact_operation_stack(
                                            &mut operator_stack,
                                            &mut operand_stack,
                                        );
                                    }
                                }
                            }
                        }
                    }
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

    impl FieldCounter for LirProgram {
        fn add_to_counter(&self, map: &mut HashMap<String, usize>) {
            for instruction in &self.instructions {
                match instruction {
                    LirInstruction::LogicalOperator(_op) => {
                        // need to do nothing here
                    }
                    LirInstruction::Predicate(p) => {
                        *map.entry(p.lhs.var_name.clone()).or_default() += 1;
                    }
                }
            }
        }

        fn remove_from_counter(&self, map: &mut HashMap<String, usize>) {
            for instruction in &self.instructions {
                match instruction {
                    LirInstruction::LogicalOperator(_op) => {
                        // need to do nothing here
                    }
                    LirInstruction::Predicate(p) => {
                        let val = map.get_mut(&p.lhs.var_name).unwrap();
                        *val -= 1;

                        if *val == 0 {
                            assert!(map.remove(&p.lhs.var_name).is_some());
                        }
                    }
                }
            }
        }
    }

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
