use crate::ast::{Expression, LogicalExpression, Predicate};
use crate::router::Router;

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
    pub fn as_predicate(&self) -> Option<&Predicate> {
        match &self {
            LirInstruction::LogicalOperator(_ops) => None, // never be here, otherwise something wrong
            LirInstruction::Predicate(p) => Some(p),
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

impl Translate for Expression {
    type Output = LirProgram;
    fn translate(&self) -> Self::Output {
        let mut lir = LirProgram::new();
        translate_helper(self, &mut lir);
        lir
    }
}

fn translate_helper(exp: &Expression, lir: &mut LirProgram) {
    match exp {
        Expression::Logical(logic_exp) => match logic_exp.as_ref() {
            LogicalExpression::And(l, r) => {
                lir.instructions
                    .push(LirInstruction::LogicalOperator(LirLogicalOperators::And));
                translate_helper(l, lir);
                translate_helper(r, lir);
            }
            LogicalExpression::Or(l, r) => {
                lir.instructions
                    .push(LirInstruction::LogicalOperator(LirLogicalOperators::Or));
                translate_helper(l, lir);
                translate_helper(r, lir);
            }
            LogicalExpression::Not(r) => {
                lir.instructions
                    .push(LirInstruction::LogicalOperator(LirLogicalOperators::Not));
                translate_helper(r, lir);
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
