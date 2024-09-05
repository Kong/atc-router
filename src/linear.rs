use crate::ast::Predicate;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct Route {
    pub stack: Vec<RouteTerm>,
}

impl Route {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub enum RouteTerm {
    LogicalOperator(RouteLogicalOperators),
    Predicate(Predicate),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub enum RouteLogicalOperators {
    And,
    Or,
    Not,
}
