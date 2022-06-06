use crate::ast::{
    BinaryOperator, Expression, LHSTransformations, LogicalExpression, Predicate, Value, LHS,
};
use crate::schema::Schema;
use regex::Regex;

type ValidationResult = Result<(), String>;

pub trait Validate {
    fn validate(&self, schema: &Schema) -> ValidationResult;
}

impl Validate for Expression {
    fn validate(&self, schema: &Schema) -> ValidationResult {
        match self {
            Expression::Logical(l) => {
                match l.as_ref() {
                    LogicalExpression::And(l, r) => {
                        l.validate(schema)?;
                        r.validate(schema)?;
                    }
                    LogicalExpression::Or(l, r) => {
                        l.validate(schema)?;
                        r.validate(schema)?;
                    }
                }

                Ok(())
            }
            Expression::Predicate(p) => {
                // lhs and rhs must be the same type
                let lhs_type = p.lhs.my_type(schema);
                if let None = lhs_type {
                    return Err("unknown LHS field".to_string());
                }

                if *lhs_type.unwrap() != p.rhs.my_type() {
                    return Err(
                        "type mismatch between the LHS and RHS values of predicate".to_string()
                    );
                }

                match p.op {
                    BinaryOperator::Equals | BinaryOperator::NotEquals => { Ok(()) }
                    BinaryOperator::Regex => {
                        match &p.rhs {
                            Value::String(r) => {
                                let r = Regex::new(r);
                                match r {
                                    Err(e) => Err(e.to_string()),
                                    Ok(_) => Ok(())
                                }
                            }
                            _ => Err("Regex/Prefix/Postfix operators only supports string operands".to_string())
                        }
                    },
                    BinaryOperator::Prefix | BinaryOperator::Postfix => {
                        match p.rhs {
                            Value::String(_) => {
                                Ok(())
                            }
                            _ => Err("Regex/Prefix/Postfix operators only supports string operands".to_string())
                        }
                    },
                    BinaryOperator::Greater | BinaryOperator::GreaterOrEqual | BinaryOperator::Lesser | BinaryOperator::LesserOrEqual => {
                        match p.rhs {
                            Value::Int(_) => {
                                Ok(())
                            }
                            _ => Err("Greater/GreaterOrEqual/Lesser/LesserOrEqual operators only supports integer operands".to_string())
                        }
                    },
                    BinaryOperator::In | BinaryOperator::NotIn => {
                        match p.rhs {
                            Value::String(_) | Value::IpCidr(_) => {
                                Ok(())
                            }
                            _ => Err("In/NotIn operators only supports string/IP cidr operands".to_string())
                        }
                    },
                }
            }
        }
    }
}
