use pest::error::LineColLocation;

use crate::ast::{
    BinaryOperator, Expression, LocationedExpression, LogicalExpression, Type, Value,
};
use crate::schema::Schema;
use std::collections::HashMap;
use std::fmt::Display;

#[derive(Debug, PartialEq, Eq)]
pub struct ValidationErr(String, Option<LineColLocation>);

impl Display for ValidationErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.1 {
            Some(LineColLocation::Pos((start, end))) => {
                write!(f, "[{}-{}] {}", start, end, self.0)
            }
            Some(LineColLocation::Span(start, end)) => {
                write!(
                    f,
                    "[{}:{}-{}:{}] {}",
                    start.0, start.1, end.0, end.1, self.0
                )
            }
            None => {
                write!(f, "{}", self.0)
            }
        }
    }
}

type ValidationResult = Result<(), ValidationErr>;

pub trait Validate {
    fn validate(&self, schema: &Schema) -> ValidationResult;
}

pub trait FieldCounter {
    fn add_to_counter(&self, map: &mut HashMap<String, usize>);
    fn remove_from_counter(&self, map: &mut HashMap<String, usize>);
}

impl Validate for LocationedExpression {
    fn validate(&self, schema: &Schema) -> ValidationResult {
        match self {
            LocationedExpression { expression, span } => expression
                .validate(schema)
                .map_err(|e| ValidationErr(e.0, Some(span.clone()))),
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
                    LogicalExpression::Not(r) => {
                        r.validate(schema)?;
                    }
                }

                Ok(())
            }
            Expression::Predicate(p) => {
                // lhs and rhs must be the same type
                let lhs_type = p.lhs.my_type(schema);
                if lhs_type.is_none() {
                    return Err(ValidationErr(String::from("Unknown LHS field"), None));
                }
                let lhs_type = lhs_type.unwrap();

                if p.op != BinaryOperator::Regex // Regex RHS is always Regex, and LHS is always String
                    && p.op != BinaryOperator::In // In/NotIn supports IPAddr in IpCidr
                    && p.op != BinaryOperator::NotIn
                    && lhs_type != &p.rhs.my_type()
                {
                    return Err(ValidationErr(
                        "Type mismatch between the LHS and RHS values of predicate".to_string(),
                        None,
                    ));
                }

                let (lower, _any) = p.lhs.get_transformations();

                // LHS transformations only makes sense with string fields
                if lower && lhs_type != &Type::String {
                    return Err(ValidationErr(
                        "lower-case transformation function only supported with String type fields"
                            .to_string(),
                        None,
                    ));
                }

                match p.op {
                    BinaryOperator::Equals | BinaryOperator::NotEquals => { Ok(()) }
                    BinaryOperator::Regex => {
                        // unchecked path above
                        if lhs_type == &Type::String {
                            Ok(())
                        } else {
                            Err(ValidationErr("Regex operators only supports string operands".to_string(),
                            None,))
                        }
                    },
                    BinaryOperator::Prefix | BinaryOperator::Postfix => {
                        match p.rhs {
                            Value::String(_) => {
                                Ok(())
                            }
                            _ => Err(ValidationErr("Regex/Prefix/Postfix operators only supports string operands".to_string(), None))
                        }
                    },
                    BinaryOperator::Greater | BinaryOperator::GreaterOrEqual | BinaryOperator::Less | BinaryOperator::LessOrEqual => {
                        match p.rhs {
                            Value::Int(_) => {
                                Ok(())
                            }
                            _ => Err(ValidationErr("Greater/GreaterOrEqual/Lesser/LesserOrEqual operators only supports integer operands".to_string(), None))
                        }
                    },
                    BinaryOperator::In | BinaryOperator::NotIn => {
                        // unchecked path above
                        match (lhs_type, &p.rhs,) {
                            (Type::IpAddr, Value::IpCidr(_)) => {
                                Ok(())
                            }
                            _ => Err(ValidationErr("In/NotIn operators only supports IP in CIDR".to_string(), None))
                        }
                    },
                    BinaryOperator::Contains => {
                        match p.rhs {
                            Value::String(_) => {
                                Ok(())
                            }
                            _ => Err(ValidationErr("Contains operator only supports string operands".to_string(), None))
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref SCHEMA: Schema = {
            let mut s = Schema::default();
            s.add_field("string", Type::String);
            s.add_field("int", Type::Int);
            s.add_field("ipaddr", Type::IpAddr);
            s
        };
    }

    #[test]
    fn unknown_field() {
        let expression = parse(r#"unkn == "abc""#).unwrap();
        assert_eq!(
            expression.validate(&SCHEMA).unwrap_err(),
            ValidationErr("Unknown LHS field".to_string(), None)
        );
    }

    #[test]
    fn string_lhs() {
        let tests = vec![
            r#"string == "abc""#,
            r#"string != "abc""#,
            r#"string ~ "abc""#,
            r#"string ^= "abc""#,
            r#"string =^ "abc""#,
            r#"lower(string) =^ "abc""#,
        ];
        for input in tests {
            let expression = parse(input).unwrap();
            expression.validate(&SCHEMA).unwrap();
        }

        let failing_tests = vec![
            r#"string == 192.168.0.1"#,
            r#"string == 192.168.0.0/24"#,
            r#"string == 123"#,
            r#"string in "abc""#,
        ];
        for input in failing_tests {
            let expression = parse(input).unwrap();
            assert!(expression.validate(&SCHEMA).is_err());
        }
    }

    #[test]
    fn ipaddr_lhs() {
        let tests = vec![
            r#"ipaddr == 192.168.0.1"#,
            r#"ipaddr == fd00::1"#,
            r#"ipaddr in 192.168.0.0/24"#,
            r#"ipaddr in fd00::/64"#,
            r#"ipaddr not in 192.168.0.0/24"#,
            r#"ipaddr not in fd00::/64"#,
        ];
        for input in tests {
            let expression = parse(input).unwrap();
            expression.validate(&SCHEMA).unwrap();
        }

        let failing_tests = vec![
            r#"ipaddr == "abc""#,
            r#"ipaddr == 123"#,
            r#"ipaddr in 192.168.0.1"#,
            r#"ipaddr in fd00::1"#,
            r#"ipaddr == 192.168.0.0/24"#,
            r#"ipaddr == fd00::/64"#,
            r#"lower(ipaddr) == fd00::1"#,
        ];
        for input in failing_tests {
            let expression = parse(input).unwrap();
            assert!(expression.validate(&SCHEMA).is_err());
        }
    }

    #[test]
    fn int_lhs() {
        let tests = vec![
            r#"int == 123"#,
            r#"int >= 123"#,
            r#"int <= 123"#,
            r#"int > 123"#,
            r#"int < 123"#,
        ];
        for input in tests {
            let expression = parse(input).unwrap();
            expression.validate(&SCHEMA).unwrap();
        }

        let failing_tests = vec![
            r#"int == "abc""#,
            r#"int in 192.168.0.0/24"#,
            r#"lower(int) == 123"#,
        ];
        for input in failing_tests {
            let expression = parse(input).unwrap();
            assert!(expression.validate(&SCHEMA).is_err());
        }
    }
}
