use crate::cir::CirInstruction;

#[derive(Debug)]
pub struct ProgramRepo {
    pub(crate) instructions: Vec<CirInstruction>,
}

impl ProgramRepo {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
        }
    }
}

impl Default for ProgramRepo {
    fn default() -> Self {
        Self::new()
    }
}
