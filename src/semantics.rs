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
        use Expression::*;
        use LogicalExpression::*;

        match self {
            Logical(l) => match l.as_ref() {
                And(l, r) | Or(l, r) => {
                    l.add_to_counter(map);
                    r.add_to_counter(map);
                }
                Not(r) => {
                    r.add_to_counter(map);
                }
            },
            Predicate(p) => {
                *map.entry(p.lhs.var_name.clone()).or_default() += 1;
            }
        }
    }

    fn remove_from_counter(&self, map: &mut HashMap<String, usize>) {
        use Expression::*;
        use LogicalExpression::*;

        match self {
            Logical(l) => match l.as_ref() {
                And(l, r) | Or(l, r) => {
                    l.remove_from_counter(map);
                    r.remove_from_counter(map);
                }
                Not(r) => {
                    r.remove_from_counter(map);
                }
            },
            Predicate(p) => {
                let val = map.get_mut(&p.lhs.var_name).unwrap();
                *val -= 1;

                if *val == 0 {
                    assert!(map.remove(&p.lhs.var_name).is_some());
                }
            }
        }
    }
}

fn raise_err(msg: &str) -> ValidationResult {
    Err(msg.to_string())
}

const MSG_UNKNOWN_LHS: &str =
    "Unknown LHS field";
const MSG_TYPE_MISMATCH_LHS_RHS: &str =
    "Type mismatch between the LHS and RHS values of predicate";
const MSG_LOWER_ONLY_FOR_STRING: &str =
    "lower-case transformation function only supported with String type fields";
const MSG_REGEX_ONLY_FOR_STRING: &str =
    "Regex operators only supports string operands";
const MSG_PREFFIX_POSTFIX_ONLY_FOR_STRING: &str =
    "Prefix/Postfix operators only supports string operands";
const MSG_ONLY_FOR_INT: &str =
    "Greater/GreaterOrEqual/Less/LessOrEqual operators only supports integer operands";
const MSG_ONLY_FOR_CIDR: &str =
    "In/NotIn operators only supports IP in CIDR";
const MSG_CONTAINS_ONLY_FOR_CIDR: &str =
    "Contains operator only supports string operands";

impl Validate for Expression {
    fn validate(&self, schema: &Schema) -> ValidationResult {
        use Expression::*;
        use LogicalExpression::*;

        match self {
            Logical(l) => {
                match l.as_ref() {
                    And(l, r) | Or(l, r) => {
                        l.validate(schema)?;
                        r.validate(schema)?;
                    }
                    Not(r) => {
                        r.validate(schema)?;
                    }
                }

                Ok(())
            }
            Predicate(p) => {
                use BinaryOperator::*;

                // lhs and rhs must be the same type
                let Some(lhs_type) = p.lhs.my_type(schema) else {
                    return raise_err(MSG_UNKNOWN_LHS);
                };

                if p.op != Regex // Regex RHS is always Regex, and LHS is always String
                    && p.op != In // In/NotIn supports IPAddr in IpCidr
                    && p.op != NotIn
                    && lhs_type != &p.rhs.my_type()
                {
                    return raise_err(MSG_TYPE_MISMATCH_LHS_RHS);
                }

                let (lower, _any) = p.lhs.get_transformations();

                // LHS transformations only makes sense with string fields
                if lower && lhs_type != &Type::String {
                    return raise_err(MSG_LOWER_ONLY_FOR_STRING);
                }

                match p.op {
                    Equals | NotEquals => { Ok(()) }
                    Regex => {
                        // unchecked path above
                        match lhs_type {
                          Type::String => {
                              Ok(())
                          }
                          _ => raise_err(MSG_REGEX_ONLY_FOR_STRING)
                        }
                    }
                    Prefix | Postfix => {
                        match p.rhs {
                            Value::String(_) => {
                                Ok(())
                            }
                            _ => raise_err(MSG_PREFFIX_POSTFIX_ONLY_FOR_STRING)
                        }
                    }
                    Greater | GreaterOrEqual | Less | LessOrEqual => {
                        match p.rhs {
                            Value::Int(_) => {
                                Ok(())
                            }
                            _ => raise_err(MSG_ONLY_FOR_INT)
                        }
                    }
                    In | NotIn => {
                        // unchecked path above
                        match (lhs_type, &p.rhs,) {
                            (Type::IpAddr, Value::IpCidr(_)) => {
                                Ok(())
                            }
                            _ => raise_err(MSG_ONLY_FOR_CIDR)
                        }
                    }
                    Contains => {
                        match p.rhs {
                            Value::String(_) => {
                                Ok(())
                            }
                            _ => raise_err(MSG_CONTAINS_ONLY_FOR_CIDR)
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
            "Unknown LHS field"
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
