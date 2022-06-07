use crate::ast::{BinaryOperator, Expression, LogicalExpression, Type, Value};
use crate::schema::Schema;

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
                if lhs_type.is_none() {
                    return Err("Unknown LHS field".to_string());
                }
                let lhs_type = lhs_type.unwrap();

                if p.op != BinaryOperator::Regex && lhs_type != &p.rhs.my_type() {
                    return Err(
                        "Type mismatch between the LHS and RHS values of predicate".to_string()
                    );
                }

                // LHS transformations only makes sense with string fields
                if p.lhs.transformation.is_some() && lhs_type != &Type::String {
                    return Err(
                        "Transformation function only supported with String type fields"
                            .to_string(),
                    );
                }

                match p.op {
                    BinaryOperator::Equals | BinaryOperator::NotEquals => { Ok(()) }
                    BinaryOperator::Regex => {
                        if lhs_type == &Type::String {
                            Ok(())
                        } else {
                            Err("Regex operators only supports string operands".to_string())
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
