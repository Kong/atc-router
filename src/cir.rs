// cir:  compact intermediate representation
use crate::ast::{Expression, LogicalExpression, Predicate};
use crate::context::{Context, Match};
use crate::interpreter::Execute;
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
    Predicate(Predicate),
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
    #[inline]
    pub fn as_predicate(&self) -> &Predicate {
        match &self {
            CirOperand::Index(_index) => {
                panic!("unexpected call to as_predicate with index operand.")
            }
            CirOperand::Predicate(p) => p,
        }
    }

    #[inline]
    pub fn as_index(&self) -> usize {
        match &self {
            CirOperand::Index(index) => *index,
            CirOperand::Predicate(_p) => {
                panic!("unexpected call to as_index with predicate operand.")
            }
        }
    }
}

#[inline]
fn is_index(cir_operand: &CirOperand) -> bool {
    match cir_operand {
        CirOperand::Index(_index) => true,
        CirOperand::Predicate(_p) => false,
    }
}

pub trait Translate {
    type Output;
    fn translate(&self) -> Self::Output;
}

impl Translate for Expression {
    type Output = CirProgram;
    fn translate(&self) -> Self::Output {
        let mut cir = CirProgram::new();
        cir_translate_helper(self, &mut cir);
        cir.instructions.shrink_to_fit(); // shrink the memory
        cir
    }
}

fn cir_translate_helper(exp: &Expression, cir: &mut CirProgram) -> usize {
    match exp {
        Expression::Logical(logic_exp) => match logic_exp.as_ref() {
            LogicalExpression::And(l, r) => {
                let left = match l {
                    Expression::Logical(_logic_exp) => {
                        CirOperand::Index(cir_translate_helper(l, cir) - 1)
                    }
                    Expression::Predicate(p) => CirOperand::Predicate(p.clone()),
                };

                let right = match r {
                    Expression::Logical(_logic_exp) => {
                        CirOperand::Index(cir_translate_helper(r, cir) - 1)
                    }
                    Expression::Predicate(p) => CirOperand::Predicate(p.clone()),
                };
                let and_ins = AndIns { left, right };
                cir.instructions.push(CirInstruction::AndIns(and_ins));
            }
            LogicalExpression::Or(l, r) => {
                let left = match l {
                    Expression::Logical(_logic_exp) => {
                        CirOperand::Index(cir_translate_helper(l, cir) - 1)
                    }
                    Expression::Predicate(p) => CirOperand::Predicate(p.clone()),
                };

                let right = match r {
                    Expression::Logical(_logic_exp) => {
                        CirOperand::Index(cir_translate_helper(r, cir) - 1)
                    }
                    Expression::Predicate(p) => CirOperand::Predicate(p.clone()),
                };
                let or_ins = OrIns { left, right };
                cir.instructions.push(CirInstruction::OrIns(or_ins));
            }
            LogicalExpression::Not(r) => {
                let right: CirOperand = match r {
                    Expression::Logical(_logic_exp) => {
                        CirOperand::Index(cir_translate_helper(r, cir) - 1)
                    }
                    Expression::Predicate(p) => CirOperand::Predicate(p.clone()),
                };
                let not_ins = NotIns { right };
                cir.instructions.push(CirInstruction::NotIns(not_ins));
            }
        },
        Expression::Predicate(p) => {
            cir.instructions.push(CirInstruction::Predicate(p.clone()));
        }
    }
    cir.instructions.len()
}

fn execute_helper(
    cir_instructions: &[CirInstruction],
    index: usize,
    ctx: &mut Context,
    m: &mut Match,
) -> bool {
    match &cir_instructions[index] {
        CirInstruction::AndIns(and) => {
            let left_val = match &and.left {
                CirOperand::Index(index) => execute_helper(cir_instructions, *index, ctx, m),
                CirOperand::Predicate(p) => p.execute(ctx, m),
            };
            left_val
                && match &and.right {
                    CirOperand::Index(index) => execute_helper(cir_instructions, *index, ctx, m),
                    CirOperand::Predicate(p) => p.execute(ctx, m),
                }
        }
        CirInstruction::OrIns(or) => {
            let left_val = match &or.left {
                CirOperand::Index(index) => execute_helper(cir_instructions, *index, ctx, m),
                CirOperand::Predicate(p) => p.execute(ctx, m),
            };
            left_val
                || match &or.right {
                    CirOperand::Index(index) => execute_helper(cir_instructions, *index, ctx, m),
                    CirOperand::Predicate(p) => p.execute(ctx, m),
                }
        }
        CirInstruction::NotIns(not) => {
            let right_val = match &not.right {
                CirOperand::Index(index) => execute_helper(cir_instructions, *index, ctx, m),
                CirOperand::Predicate(p) => p.execute(ctx, m),
            };
            !right_val
        }
        CirInstruction::Predicate(p) => p.execute(ctx, m),
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
                        *map.entry(and.left.as_predicate().lhs.var_name.clone())
                            .or_default() += 1;
                    }
                    if !is_index(&and.right) {
                        *map.entry(and.right.as_predicate().lhs.var_name.clone())
                            .or_default() += 1;
                    }
                }
                CirInstruction::OrIns(or) => {
                    if !is_index(&or.left) {
                        *map.entry(or.left.as_predicate().lhs.var_name.clone())
                            .or_default() += 1;
                    }
                    if !is_index(&or.right) {
                        *map.entry(or.right.as_predicate().lhs.var_name.clone())
                            .or_default() += 1;
                    }
                }
                CirInstruction::NotIns(not) => {
                    if !is_index(&not.right) {
                        *map.entry(not.right.as_predicate().lhs.var_name.clone())
                            .or_default() += 1;
                    }
                }
                CirInstruction::Predicate(p) => {
                    *map.entry(p.lhs.var_name.clone()).or_default() += 1;
                }
            }
        }
    }

    fn remove_from_counter(&self, map: &mut HashMap<String, usize>) {
        for instruction in &self.instructions {
            match &instruction {
                CirInstruction::AndIns(and) => {
                    if !is_index(&and.left) {
                        let left = and.left.as_predicate();
                        let val = map.get_mut(&left.lhs.var_name).unwrap();
                        *val -= 1;

                        if *val == 0 {
                            assert!(map.remove(&left.lhs.var_name).is_some());
                        }
                    }

                    if !is_index(&and.right) {
                        let right = and.right.as_predicate();
                        let val = map.get_mut(&right.lhs.var_name).unwrap();
                        *val -= 1;

                        if *val == 0 {
                            assert!(map.remove(&right.lhs.var_name).is_some());
                        }
                    }
                }
                CirInstruction::OrIns(or) => {
                    if !is_index(&or.left) {
                        let left = or.left.as_predicate();
                        let val = map.get_mut(&left.lhs.var_name).unwrap();
                        *val -= 1;

                        if *val == 0 {
                            assert!(map.remove(&left.lhs.var_name).is_some());
                        }
                    }

                    if !is_index(&or.right) {
                        let right = or.right.as_predicate();
                        let val = map.get_mut(&right.lhs.var_name).unwrap();
                        *val -= 1;

                        if *val == 0 {
                            assert!(map.remove(&right.lhs.var_name).is_some());
                        }
                    }
                }
                CirInstruction::NotIns(not) => {
                    if !is_index(&not.right) {
                        let right = not.right.as_predicate();
                        let val = map.get_mut(&right.lhs.var_name).unwrap();
                        *val -= 1;

                        if *val == 0 {
                            assert!(map.remove(&right.lhs.var_name).is_some());
                        }
                    }
                }
                CirInstruction::Predicate(p) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Expression;
    use crate::context::Match;
    use crate::interpreter::Execute;
    use crate::schema::Schema;

    impl Execute for Expression {
        fn execute(&self, ctx: &mut Context, m: &mut Match) -> bool {
            use crate::ast::{Expression, LogicalExpression};
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

    #[test]
    fn verify_translate_execute() {
        let mut schema = Schema::default();
        schema.add_field("a", crate::ast::Type::Int);
        schema.add_field("http.path", crate::ast::Type::String);
        schema.add_field("http.version", crate::ast::Type::String);

        let sources = vec![
            r#"a == 5 "#,
            r#"!(!(a == 1 && a == 2) || a == 3 && !(a == 4))"#,
            r#"!(( a == 2) && ( a == 9 )) || !(a == 1) || (http.path == "hello" && http.version == "1.1") || ( a == 3 && a == 4) && !(a == 5)"#,
            r#"(http.path == "hello" && http.version == "1.1") || !(( a == 2) && ( a == 9 )) || !(a == 1) || ( a == 5 && a == 4) && !(a == 3)"#,
            r#"(http.path == "hello" && http.version == "1.1") || ( a == 3 && a == 4) && !(a == 5)"#,
            r#"http.path == "hello" && http.version == "1.1""#,
        ];

        let mut context = crate::context::Context::new(&schema);
        context.add_value("http.path", crate::ast::Value::String("hello".to_string()));
        context.add_value("http.version", crate::ast::Value::String("1.1".to_string()));

        for source in sources {
            let ast = crate::parser::parse(source)
                .map_err(|e| e.to_string())
                .unwrap();
            let mut mat = Match::new();
            let ast_result = ast.execute(&mut context, &mut mat);

            let cir_result = ast.translate().execute(&mut context, &mut mat);
            assert_eq!(ast_result, cir_result);
        }
    }
}
