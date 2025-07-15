// cir:  compact intermediate representation
use crate::ast::{Expression, LogicalExpression, Predicate};
//use crate::context::{Context, Match};
//use crate::interpreter::Execute;
//use crate::semantics::FieldCounter;
//use std::collections::HashMap;

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
    And(CirOperand, CirOperand),
    Or(CirOperand, CirOperand),
    Not(CirOperand),
    Predicate(Predicate),
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
    use Expression::{Logical, Predicate};
    use LogicalExpression::{And, Not, Or};

    match exp {
        Logical(logic_exp) => match logic_exp.as_ref() {
            And(l, r) => {
                let left = match l {
                    Logical(_) => CirOperand::Index(cir_translate_helper(l, cir)),
                    Predicate(p) => CirOperand::Predicate(p.clone()),
                };

                let right = match r {
                    Logical(_) => CirOperand::Index(cir_translate_helper(r, cir)),
                    Predicate(p) => CirOperand::Predicate(p.clone()),
                };

                cir.instructions.push(CirInstruction::And(left, right));
            }
            Or(l, r) => {
                let left = match l {
                    Logical(_) => CirOperand::Index(cir_translate_helper(l, cir)),
                    Predicate(p) => CirOperand::Predicate(p.clone()),
                };

                let right = match r {
                    Logical(_) => CirOperand::Index(cir_translate_helper(r, cir)),
                    Predicate(p) => CirOperand::Predicate(p.clone()),
                };
                cir.instructions.push(CirInstruction::Or(left, right));
            }
            Not(r) => {
                let right: CirOperand = match r {
                    Logical(_) => CirOperand::Index(cir_translate_helper(r, cir)),
                    Predicate(p) => CirOperand::Predicate(p.clone()),
                };
                cir.instructions.push(CirInstruction::Not(right));
            }
        },
        Predicate(p) => {
            cir.instructions.push(CirInstruction::Predicate(p.clone()));
        }
    }

    cir.instructions.len() - 1
}
