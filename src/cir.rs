// cir:  compact intermediate representation
use crate::ast::{Expression, LogicalExpression, Predicate};
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::router::Fields;
use crate::semantics::FieldCounter;

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

/// Helper function for translation from AST to CIR.
/// Parameters:
///   * reference to AST
///   * reference to translated CIR
///     This function returns:
///   * index of translated IR
fn cir_translate_helper(exp: &Expression, cir: &mut CirProgram) -> usize {
    match exp {
        Expression::Logical(logic_exp) => match logic_exp.as_ref() {
            LogicalExpression::And(l, r) => {
                let left = match l {
                    Expression::Logical(_logic_exp) => {
                        CirOperand::Index(cir_translate_helper(l, cir))
                    }
                    Expression::Predicate(p) => CirOperand::Predicate(p.clone()),
                };

                let right = match r {
                    Expression::Logical(_logic_exp) => {
                        CirOperand::Index(cir_translate_helper(r, cir))
                    }
                    Expression::Predicate(p) => CirOperand::Predicate(p.clone()),
                };
                let and_ins = AndIns { left, right };
                cir.instructions.push(CirInstruction::AndIns(and_ins));
            }
            LogicalExpression::Or(l, r) => {
                let left = match l {
                    Expression::Logical(_logic_exp) => {
                        CirOperand::Index(cir_translate_helper(l, cir))
                    }
                    Expression::Predicate(p) => CirOperand::Predicate(p.clone()),
                };

                let right = match r {
                    Expression::Logical(_logic_exp) => {
                        CirOperand::Index(cir_translate_helper(r, cir))
                    }
                    Expression::Predicate(p) => CirOperand::Predicate(p.clone()),
                };
                let or_ins = OrIns { left, right };
                cir.instructions.push(CirInstruction::OrIns(or_ins));
            }
            LogicalExpression::Not(r) => {
                let right: CirOperand = match r {
                    Expression::Logical(_logic_exp) => {
                        CirOperand::Index(cir_translate_helper(r, cir))
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
    cir.instructions.len() - 1
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

impl FieldCounter for CirOperand {
    fn add_to_counter(&mut self, fields: &mut Fields) {
        if let CirOperand::Predicate(p) = self {
            // 1. fields: increment counter for field
            // 2. lhs: assign field index to the LHS
            // 3. map: maintain the fields map: {field_name : field_index}
            if let Some(index) = fields.map.get(&p.lhs.var_name) {
                fields.list[*index].as_mut().unwrap().1 += 1;
                p.lhs.index = *index;
            } else {
                // reuse slots in queue if possible
                let new_idx: usize;
                if fields.slots.is_empty() {
                    fields.list.push(Some((p.lhs.var_name.clone(), 1)));
                    new_idx = fields.list.len() - 1;
                } else {
                    new_idx = fields.slots.pop().unwrap();
                    fields.list[new_idx] = Some((p.lhs.var_name.clone(), 1));
                }
                fields.map.insert(p.lhs.var_name.clone(), new_idx);
                p.lhs.index = new_idx;
            }
        }
    }
    fn remove_from_counter(&mut self, fields: &mut Fields) {
        if let CirOperand::Predicate(p) = &self {
            let index: usize = p.lhs.index;
            // decrement counter of field
            fields.list[index].as_mut().unwrap().1 -= 1;
            // for field removing, reserve the slot for resue and remove it in map
            if fields.list[index].as_mut().unwrap().1 == 0 {
                fields.list[index] = None;
                fields.slots.push(index);
                assert!(fields.map.remove(&p.lhs.var_name).is_some());
            }
        }
    }
}

impl FieldCounter for CirInstruction {
    fn add_to_counter(&mut self, fields: &mut Fields) {
        match self {
            CirInstruction::AndIns(and) => {
                and.left.add_to_counter(fields);
                and.right.add_to_counter(fields);
            }
            CirInstruction::OrIns(or) => {
                or.left.add_to_counter(fields);
                or.right.add_to_counter(fields);
            }
            CirInstruction::NotIns(not) => {
                not.right.add_to_counter(fields);
            }
            CirInstruction::Predicate(p) => {
                // 1. fields: increment counter for field
                // 2. lhs: assign field index to the LHS
                // 3. map: maintain the fields map: {field_name : field_index}
                if let Some(index) = fields.map.get(&p.lhs.var_name) {
                    fields.list[*index].as_mut().unwrap().1 += 1;
                    p.lhs.index = *index;
                } else {
                    // reuse slots in queue if possible
                    let new_idx: usize;
                    if fields.slots.is_empty() {
                        fields.list.push(Some((p.lhs.var_name.clone(), 1)));
                        new_idx = fields.list.len() - 1;
                    } else {
                        new_idx = fields.slots.pop().unwrap();
                        fields.list[new_idx] = Some((p.lhs.var_name.clone(), 1));
                    }
                    fields.map.insert(p.lhs.var_name.clone(), new_idx);
                    p.lhs.index = new_idx;
                }
            }
        }
    }
    fn remove_from_counter(&mut self, fields: &mut Fields) {
        match self {
            CirInstruction::AndIns(and) => {
                and.left.remove_from_counter(fields);
                and.right.remove_from_counter(fields);
            }
            CirInstruction::OrIns(or) => {
                or.left.remove_from_counter(fields);
                or.right.remove_from_counter(fields);
            }
            CirInstruction::NotIns(not) => {
                not.right.remove_from_counter(fields);
            }
            CirInstruction::Predicate(p) => {
                let index: usize = p.lhs.index;
                // decrement counter of field
                fields.list[index].as_mut().unwrap().1 -= 1;
                // for field removing, reserve the slot for resue and remove it in map
                if fields.list[index].as_mut().unwrap().1 == 0 {
                    fields.list[index] = None;
                    fields.slots.push(index);
                    assert!(fields.map.remove(&p.lhs.var_name).is_some());
                }
            }
        }
    }
}

impl FieldCounter for CirProgram {
    fn add_to_counter(&mut self, fields: &mut Fields) {
        self.instructions
            .iter_mut()
            .for_each(|instruction: &mut CirInstruction| instruction.add_to_counter(fields));
    }

    fn remove_from_counter(&mut self, fields: &mut Fields) {
        self.instructions
            .iter_mut()
            .for_each(|instruction: &mut CirInstruction| instruction.remove_from_counter(fields));
    }
}

#[cfg(test)]
pub fn get_predicates(cir: &CirProgram) -> Vec<&Predicate> {
    let mut predicates = Vec::new();
    cir.instructions.iter().for_each(|ins| match ins {
        CirInstruction::AndIns(and) => {
            if let CirOperand::Predicate(predicate) = &and.left {
                predicates.push(predicate);
            }
            if let CirOperand::Predicate(predicate) = &and.right {
                predicates.push(predicate);
            }
        }
        CirInstruction::OrIns(or) => {
            if let CirOperand::Predicate(predicate) = &or.left {
                predicates.push(predicate);
            }
            if let CirOperand::Predicate(predicate) = &or.right {
                predicates.push(predicate);
            }
        }
        CirInstruction::NotIns(not) => {
            if let CirOperand::Predicate(predicate) = &not.right {
                predicates.push(predicate);
            }
        }
        CirInstruction::Predicate(predicate) => {
            predicates.push(predicate);
        }
    });
    predicates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Expression;
    use crate::ast::Value;
    use crate::context::Match;
    use crate::interpreter::Execute;
    use crate::router::Router;
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

        let r = Router::new(&schema);
        let mut context = crate::context::Context::new(&r);
        context.add_value("http.path", crate::ast::Value::String("hello".to_string()));
        context.add_value("http.version", crate::ast::Value::String("1.1".to_string()));
        context.add_value("a", Value::Int(3 as i64));

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
