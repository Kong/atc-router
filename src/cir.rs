// cir:  compact intermediate representation
use crate::ast::{Expression, LogicalExpression, Predicate};
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::semantics::{FieldCounter, ValidationHashMap};

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

fn execute_helper(
    cir_instructions: &[CirInstruction],
    index: usize,
    ctx: &Context,
    m: &mut Match,
) -> bool {
    match &cir_instructions[index] {
        CirInstruction::And(left, right) => {
            let left_val = match &left {
                CirOperand::Index(index) => execute_helper(cir_instructions, *index, ctx, m),
                CirOperand::Predicate(p) => p.execute(ctx, m),
            };
            left_val
                && match &right {
                    CirOperand::Index(index) => execute_helper(cir_instructions, *index, ctx, m),
                    CirOperand::Predicate(p) => p.execute(ctx, m),
                }
        }
        CirInstruction::Or(left, right) => {
            let left_val = match &left {
                CirOperand::Index(index) => execute_helper(cir_instructions, *index, ctx, m),
                CirOperand::Predicate(p) => p.execute(ctx, m),
            };
            left_val
                || match &right {
                    CirOperand::Index(index) => execute_helper(cir_instructions, *index, ctx, m),
                    CirOperand::Predicate(p) => p.execute(ctx, m),
                }
        }
        CirInstruction::Not(right) => {
            let right_val = match &right {
                CirOperand::Index(index) => execute_helper(cir_instructions, *index, ctx, m),
                CirOperand::Predicate(p) => p.execute(ctx, m),
            };
            !right_val
        }
        CirInstruction::Predicate(p) => p.execute(ctx, m),
    }
}

impl Execute for CirProgram {
    fn execute(&self, ctx: &Context, m: &mut Match) -> bool {
        execute_helper(&self.instructions, self.instructions.len() - 1, ctx, m)
    }
}

impl FieldCounter for CirOperand {
    fn add_to_counter(&self, map: &mut ValidationHashMap) {
        if let CirOperand::Predicate(p) = &self {
            *map.entry(p.lhs.var_name.clone()).or_default() += 1
        }
    }
    fn remove_from_counter(&self, map: &mut ValidationHashMap) {
        if let CirOperand::Predicate(p) = &self {
            let val = map.get_mut(&p.lhs.var_name).unwrap();
            *val -= 1;

            if *val == 0 {
                assert!(map.remove(&p.lhs.var_name).is_some());
            }
        }
    }
}

impl FieldCounter for CirInstruction {
    fn add_to_counter(&self, map: &mut ValidationHashMap) {
        match self {
            CirInstruction::And(left, right) => {
                left.add_to_counter(map);
                right.add_to_counter(map);
            }
            CirInstruction::Or(left, right) => {
                left.add_to_counter(map);
                right.add_to_counter(map);
            }
            CirInstruction::Not(right) => {
                right.add_to_counter(map);
            }
            CirInstruction::Predicate(p) => {
                *map.entry(p.lhs.var_name.clone()).or_default() += 1;
            }
        }
    }
    fn remove_from_counter(&self, map: &mut ValidationHashMap) {
        match self {
            CirInstruction::And(left, right) => {
                left.remove_from_counter(map);
                right.remove_from_counter(map);
            }
            CirInstruction::Or(left, right) => {
                left.remove_from_counter(map);
                right.remove_from_counter(map);
            }
            CirInstruction::Not(right) => {
                right.remove_from_counter(map);
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

impl FieldCounter for CirProgram {
    fn add_to_counter(&self, map: &mut ValidationHashMap) {
        self.instructions
            .iter()
            .for_each(|instruction: &CirInstruction| instruction.add_to_counter(map));
    }

    fn remove_from_counter(&self, map: &mut ValidationHashMap) {
        self.instructions
            .iter()
            .for_each(|instruction: &CirInstruction| instruction.remove_from_counter(map));
    }
}
