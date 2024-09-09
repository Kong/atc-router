use crate::ast::{BinaryOperator, Expression, LogicalExpression, Type, Value};
use crate::router::Fields;
use crate::schema::Schema;

type ValidationResult = Result<(), String>;

pub trait Validate {
    fn validate(&self, schema: &Schema) -> ValidationResult;
}

pub trait FieldCounter {
    fn add_to_counter(&mut self, fields: &mut Fields);
    fn remove_from_counter(&mut self, fields: &mut Fields);
}

impl FieldCounter for Expression {
    fn add_to_counter(&mut self, fields: &mut Fields) {
        match self {
            Expression::Logical(l) => match l.as_mut() {
                LogicalExpression::And(l, r) => {
                    l.add_to_counter(fields);
                    r.add_to_counter(fields);
                }
                LogicalExpression::Or(l, r) => {
                    l.add_to_counter(fields);
                    r.add_to_counter(fields);
                }
                LogicalExpression::Not(r) => {
                    r.add_to_counter(fields);
                }
            },
            Expression::Predicate(p) => {
                // 1. fields: increment counter for field
                // 2. lhs: assign field index to the LHS
                // 3. map: maintain the fields map: {field_name : field_index}
                if let Some(index) = fields.map.get(&p.lhs.var_name) {
                    fields.list[*index].as_mut().unwrap().1 += 1;
                    p.lhs.index = *index;
                } else {
                    // reuse slots in queue if possible
                    let new_idx: usize;
                    if fields.slots.is_empty() {
                        fields.list.push(Some((p.lhs.var_name.clone(), 1)));
                        new_idx = fields.list.len() - 1;
                    } else {
                        new_idx = fields.slots.pop().unwrap();
                        fields.list[new_idx] = Some((p.lhs.var_name.clone(), 1));
                    }
                    fields.map.insert(p.lhs.var_name.clone(), new_idx);
                    p.lhs.index = new_idx;
                }
            }
        }
    }

    fn remove_from_counter(&mut self, fields: &mut Fields) {
        match self {
            Expression::Logical(l) => match l.as_mut() {
                LogicalExpression::And(l, r) => {
                    l.remove_from_counter(fields);
                    r.remove_from_counter(fields);
                }
                LogicalExpression::Or(l, r) => {
                    l.remove_from_counter(fields);
                    r.remove_from_counter(fields);
                }
                LogicalExpression::Not(r) => {
                    r.remove_from_counter(fields);
                }
            },
            Expression::Predicate(p) => {
                let index: usize = p.lhs.index;
                // decrement counter of field
                fields.list[index].as_mut().unwrap().1 -= 1;
                // for field removing, reserve the slot for resue and remove it in map
                if fields.list[index].as_mut().unwrap().1 == 0 {
                    fields.list[index] = None;
                    fields.slots.push(index);
                    assert!(fields.map.remove(&p.lhs.var_name).is_some());
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
                        // unchecked path above
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
                        // unchecked path above
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
