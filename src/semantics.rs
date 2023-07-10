use crate::ast::{BinaryOperator, Expression, LogicalExpression, Type, Value};
use crate::schema::Schema;
use std::collections::HashMap;

type ValidationResult = Result<(), String>;

pub trait Validate {
    fn validate(&self, schema: &Schema) -> ValidationResult;
}

pub trait FieldCounter {
    fn add_to_counter(&self, map: &mut HashMap<String, usize>);
    fn remove_from_counter(&self, map: &mut HashMap<String, usize>);
}

impl FieldCounter for Expression {
    fn add_to_counter(&self, map: &mut HashMap<String, usize>) {
        match self {
            Expression::Logical(l) => match l.as_ref() {
                LogicalExpression::And(l, r) => {
                    l.add_to_counter(map);
                    r.add_to_counter(map);
                }
                LogicalExpression::Or(l, r) => {
                    l.add_to_counter(map);
                    r.add_to_counter(map);
                }
            },
            Expression::Predicate(p) => {
                *map.entry(p.lhs.var_name.clone()).or_default() += 1;
            }
        }
    }

    fn remove_from_counter(&self, map: &mut HashMap<String, usize>) {
        match self {
            Expression::Logical(l) => match l.as_ref() {
                LogicalExpression::And(l, r) => {
                    l.remove_from_counter(map);
                    r.remove_from_counter(map);
                }
                LogicalExpression::Or(l, r) => {
                    l.remove_from_counter(map);
                    r.remove_from_counter(map);
                }
            },
            Expression::Predicate(p) => {
                let val = map.get_mut(&p.lhs.var_name).unwrap();
                *val -= 1;

                if *val == 0 {
                    assert!(map.remove(&p.lhs.var_name).is_some());
                }
            }
        }
    }
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

                if p.op != BinaryOperator::Regex // Regex RHS is always Regex, and LHS is always String
                    && p.op != BinaryOperator::In // In/NotIn supports IPAddr in IpCidr
                    && p.op != BinaryOperator::NotIn
                    && lhs_type != &p.rhs.my_type()
                {
                    return Err(
                        "Type mismatch between the LHS and RHS values of predicate".to_string()
                    );
                }

                let (lower, _any) = p.lhs.get_transformations();

                // LHS transformations only makes sense with string fields
                if lower && lhs_type != &Type::String {
                    return Err(
                        "lower-case transformation function only supported with String type fields"
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
                    BinaryOperator::Greater | BinaryOperator::GreaterOrEqual | BinaryOperator::Less | BinaryOperator::LessOrEqual => {
                        match p.rhs {
                            Value::Int(_) => {
                                Ok(())
                            }
                            _ => Err("Greater/GreaterOrEqual/Lesser/LesserOrEqual operators only supports integer operands".to_string())
                        }
                    },
                    BinaryOperator::In | BinaryOperator::NotIn => {
                        match (lhs_type, &p.rhs,) {
                            (Type::IpAddr, Value::IpCidr(_)) => {
                                Ok(())
                            }
                            _ => Err("In/NotIn operators only supports IP in CIDR".to_string())
                        }
                    },
                    BinaryOperator::Contains => {
                        match p.rhs {
                            Value::String(_) => {
                                Ok(())
                            }
                            _ => Err("Contains operator only supports string operands".to_string())
                        }
                    }
                }
            }
        }
    }
}
