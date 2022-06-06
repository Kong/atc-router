use crate::schema::Schema;
use cidr::IpCidr;
use regex::Regex;
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

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum Value {
    String(String),
    IpCidr(IpCidr),
    Int(i64),
}

impl Value {
    pub fn my_type(&self) -> Type {
        match self {
            Value::String(_) => Type::String,
            Value::IpCidr(_) => Type::IpCidr,
            Value::Int(_) => Type::Int,
        }
    }
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[repr(C)]
pub enum Type {
    String,
    IpCidr,
    Int,
}

#[derive(Debug, Serialize)]
pub struct LHS {
    pub var_name: String,
    pub transformation: Option<LHSTransformations>,
}

impl LHS {
    pub fn my_type<'a>(&self, schema: &'a Schema) -> Option<&'a Type> {
        schema.type_of(&self.var_name)
    }
}

#[derive(Debug, Serialize)]
pub struct Predicate {
    pub lhs: LHS,
    pub rhs: Value,
    pub op: BinaryOperator,
}
