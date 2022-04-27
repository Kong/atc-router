use cidr::IpCidr;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "expression")]
pub enum Expression {
    Logical(Box<LogicalExpression>),
    Predicate(Predicate),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "expressions")]
pub enum LogicalExpression {
    And(Expression, Expression),
    Or(Expression, Expression),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum LHSTransformations {
    Lower,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum BinaryOperator {
    Equals,         // ==
    NotEquals,      // !=
    Regex,          // ~
    Prefix,         // ^=
    Postfix,        // =^
    Greater,        // >
    GreaterOrEqual, // >=
    Lesser,         // <
    LesserOrEqual,  // <=
    In,             // in
    NotIn,          // not in
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum RHS {
    String(String),
    IpCidr(IpCidr),
    Int(i64),
}

#[derive(Debug, Serialize)]
pub struct LHS {
    pub var_name: String,
    pub transformation: Option<LHSTransformations>,
}

#[derive(Debug, Serialize)]
pub struct Predicate {
    pub lhs: LHS,
    pub rhs: RHS,
    pub op: BinaryOperator,
}
