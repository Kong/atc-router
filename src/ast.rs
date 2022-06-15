use crate::schema::Schema;
use cidr::IpCidr;
use regex::Regex;
use std::net::IpAddr;

#[derive(Debug)]
pub enum Expression {
    Logical(Box<LogicalExpression>),
    Predicate(Predicate),
}

#[derive(Debug)]
pub enum LogicalExpression {
    And(Expression, Expression),
    Or(Expression, Expression),
}

#[derive(Debug, PartialEq, Eq)]
pub enum LhsTransformations {
    Lower,
    Any,
}

#[derive(Debug, PartialEq)]
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

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    IpCidr(IpCidr),
    IpAddr(IpAddr),
    Int(i64),
    Regex(Regex),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Regex(_), _) | (_, Self::Regex(_)) => {
                panic!("Regexes can not be compared using eq")
            }
            (Self::String(s1), Self::String(s2)) => s1 == s2,
            (Self::IpCidr(i1), Self::IpCidr(i2)) => i1 == i2,
            (Self::Int(i1), Self::Int(i2)) => i1 == i2,
            _ => false,
        }
    }
}

impl Value {
    pub fn my_type(&self) -> Type {
        match self {
            Value::String(_) => Type::String,
            Value::IpCidr(_) => Type::IpCidr,
            Value::IpAddr(_) => Type::IpAddr,
            Value::Int(_) => Type::Int,
            Value::Regex(_) => Type::Regex,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub enum Type {
    String,
    IpCidr,
    IpAddr,
    Int,
    Regex,
}

#[derive(Debug)]
pub struct Lhs {
    pub var_name: String,
    pub transformations: Vec<LhsTransformations>,
}

impl Lhs {
    pub fn my_type<'a>(&self, schema: &'a Schema) -> Option<&'a Type> {
        schema.type_of(&self.var_name)
    }

    pub fn get_transformations(&self) -> (bool, bool) {
        let mut lower = false;
        let mut any = false;

        self.transformations.iter().for_each(|i| match i {
            LhsTransformations::Any => any = true,
            LhsTransformations::Lower => lower = true,
        });

        (lower, any)
    }
}

#[derive(Debug)]
pub struct Predicate {
    pub lhs: Lhs,
    pub rhs: Value,
    pub op: BinaryOperator,
}
