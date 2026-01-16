use crate::schema::Schema;
use cidr::IpCidr;
use regex::Regex;
use std::net::IpAddr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expression {
    Logical(Box<LogicalExpression>),
    Predicate(Predicate),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogicalExpression {
    And(Expression, Expression),
    Or(Expression, Expression),
    Not(Expression),
}

bitflags::bitflags! {
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct LhsTransformations: u8 {
        const LOWER = 1 << 0;
        const ANY = 1 << 1;
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    Equals,         // ==
    NotEquals,      // !=
    Regex,          // ~
    Prefix,         // ^=
    Postfix,        // =^
    Greater,        // >
    GreaterOrEqual, // >=
    Less,           // <
    LessOrEqual,    // <=
    In,             // in
    NotIn,          // not in
    Contains,       // contains
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    IpCidr(IpCidr),
    IpAddr(IpAddr),
    Int(i64),
    #[cfg_attr(feature = "serde", serde(with = "serde_regex"))]
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
            (Self::IpAddr(i1), Self::IpAddr(i2)) => i1 == i2,
            (Self::Int(i1), Self::Int(i2)) => i1 == i2,
            _ => false,
        }
    }
}

impl Eq for Value {}

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

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        let Value::String(s) = self else {
            return None;
        };
        Some(s.as_str())
    }

    pub fn as_regex(&self) -> Option<&Regex> {
        let Value::Regex(r) = self else {
            return None;
        };
        Some(r)
    }

    pub fn as_int(&self) -> Option<i64> {
        let Value::Int(i) = self else {
            return None;
        };
        Some(*i)
    }

    pub fn as_ipaddr(&self) -> Option<&IpAddr> {
        let Value::IpAddr(a) = self else {
            return None;
        };
        Some(a)
    }

    pub fn as_ipcidr(&self) -> Option<&IpCidr> {
        let Value::IpCidr(c) = self else {
            return None;
        };
        Some(c)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub enum Type {
    String,
    IpCidr,
    IpAddr,
    Int,
    Regex,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lhs {
    pub var_name: String,
    pub transformations: LhsTransformations,
}

impl Lhs {
    pub fn my_type<'a>(&self, schema: &'a Schema) -> Option<&'a Type> {
        schema.type_of(&self.var_name)
    }

    pub fn get_transformations(&self) -> (bool, bool) {
        let lower = self.transformations.contains(LhsTransformations::LOWER);
        let any = self.transformations.contains(LhsTransformations::ANY);

        (lower, any)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Predicate {
    pub lhs: Lhs,
    pub rhs: Value,
    pub op: BinaryOperator,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use std::fmt;

    impl fmt::Display for Expression {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    Expression::Logical(logical) => logical.to_string(),
                    Expression::Predicate(predicate) => predicate.to_string(),
                }
            )
        }
    }

    impl fmt::Display for LogicalExpression {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    LogicalExpression::And(left, right) => {
                        format!("({} && {})", left, right)
                    }
                    LogicalExpression::Or(left, right) => {
                        format!("({} || {})", left, right)
                    }
                    LogicalExpression::Not(e) => {
                        format!("!({})", e)
                    }
                }
            )
        }
    }

    impl fmt::Display for LhsTransformations {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "{}",
                match *self {
                    LhsTransformations::LOWER => "lower",
                    LhsTransformations::ANY => "any",
                    _ => "lower|any",
                }
            )
        }
    }

    impl fmt::Display for Value {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Value::String(s) => write!(f, "\"{}\"", s),
                Value::IpCidr(cidr) => write!(f, "{}", cidr),
                Value::IpAddr(addr) => write!(f, "{}", addr),
                Value::Int(i) => write!(f, "{}", i),
                Value::Regex(re) => write!(f, "\"{}\"", re),
            }
        }
    }

    impl fmt::Display for Lhs {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let mut s = self.var_name.to_string();
            for (transform_name, _) in self.transformations.iter_names() {
                s = format!("{}({})", transform_name, s);
            }
            write!(f, "{}", s)
        }
    }

    impl fmt::Display for BinaryOperator {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            use BinaryOperator::*;

            write!(
                f,
                "{}",
                match self {
                    Equals => "==",
                    NotEquals => "!=",
                    Regex => "~",
                    Prefix => "^=",
                    Postfix => "=^",
                    Greater => ">",
                    GreaterOrEqual => ">=",
                    Less => "<",
                    LessOrEqual => "<=",
                    In => "in",
                    NotIn => "not in",
                    Contains => "contains",
                }
            )
        }
    }

    impl fmt::Display for Predicate {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "({} {} {})", self.lhs, self.op, self.rhs)
        }
    }

    #[test]
    fn expr_op_and_prec() {
        let tests = vec![
            ("a > 0", "(a > 0)"),
            ("a in \"abc\"", "(a in \"abc\")"),
            ("a == 1 && b != 2", "((a == 1) && (b != 2))"),
            (
                "a ^= \"1\" && b =^ \"2\" || c >= 3",
                "((a ^= \"1\") && ((b =^ \"2\") || (c >= 3)))",
            ),
            (
                "a == 1 && b != 2 || c >= 3",
                "((a == 1) && ((b != 2) || (c >= 3)))",
            ),
            (
                "a > 1 || b < 2 && c <= 3 || d not in \"foo\"",
                "(((a > 1) || (b < 2)) && ((c <= 3) || (d not in \"foo\")))",
            ),
            (
                "a > 1 || ((b < 2) && (c <= 3)) || d not in \"foo\"",
                "(((a > 1) || ((b < 2) && (c <= 3))) || (d not in \"foo\"))",
            ),
            ("!(a == 1)", "!((a == 1))"),
            (
                "!(a == 1) && b == 2 && !(c == 3) && d >= 4",
                "(((!((a == 1)) && (b == 2)) && !((c == 3))) && (d >= 4))",
            ),
            (
                "!(a == 1 || b == 2 && c == 3) && d == 4",
                "(!((((a == 1) || (b == 2)) && (c == 3))) && (d == 4))",
            ),
        ];
        for (input, expected) in tests {
            let result = parse(input).unwrap();
            assert_eq!(result.to_string(), expected);
        }
    }

    #[test]
    fn expr_var_name_and_ip() {
        let tests = vec![
            // ipv4_literal
            ("kong.foo in 1.1.1.1", "(kong.foo in 1.1.1.1)"),
            // ipv4_cidr_literal
            (
                "kong.foo.foo2 in 10.0.0.0/24",
                "(kong.foo.foo2 in 10.0.0.0/24)",
            ),
            // ipv6_literal
            (
                "kong.foo.foo3 in 2001:db8::/32",
                "(kong.foo.foo3 in 2001:db8::/32)",
            ),
            // ipv6_cidr_literal
            (
                "kong.foo.foo4 in 2001:db8::/32",
                "(kong.foo.foo4 in 2001:db8::/32)",
            ),
        ];
        for (input, expected) in tests {
            let result = parse(input).unwrap();
            assert_eq!(result.to_string(), expected);
        }
    }

    #[test]
    fn expr_regex() {
        let tests = vec![
            // regex_literal
            (
                "kong.foo.foo5 ~ \"^foo.*$\"",
                "(kong.foo.foo5 ~ \"^foo.*$\")",
            ),
            // regex_literal
            (
                "kong.foo.foo6 ~ \"^foo.*$\"",
                "(kong.foo.foo6 ~ \"^foo.*$\")",
            ),
        ];
        for (input, expected) in tests {
            let result = parse(input).unwrap();
            assert_eq!(result.to_string(), expected);
        }
    }

    #[test]
    fn expr_digits() {
        let tests = vec![
            // dec literal
            ("kong.foo.foo7 == 123", "(kong.foo.foo7 == 123)"),
            // hex literal
            ("kong.foo.foo8 == 0x123", "(kong.foo.foo8 == 291)"),
            // oct literal
            ("kong.foo.foo9 == 0123", "(kong.foo.foo9 == 83)"),
            // dec negative literal
            ("kong.foo.foo10 == -123", "(kong.foo.foo10 == -123)"),
            // hex negative literal
            ("kong.foo.foo11 == -0x123", "(kong.foo.foo11 == -291)"),
            // oct negative literal
            ("kong.foo.foo12 == -0123", "(kong.foo.foo12 == -83)"),
        ];
        for (input, expected) in tests {
            let result = parse(input).unwrap();
            assert_eq!(result.to_string(), expected);
        }
    }

    #[test]
    fn expr_transformations() {
        let tests = vec![
            // lower
            (
                "lower(kong.foo.foo13) == \"foo\"",
                "(lower(kong.foo.foo13) == \"foo\")",
            ),
            // any
            (
                "any(kong.foo.foo14) == \"foo\"",
                "(any(kong.foo.foo14) == \"foo\")",
            ),
        ];
        for (input, expected) in tests {
            let result = parse(input).unwrap();
            assert_eq!(result.to_string(), expected);
        }
    }

    #[test]
    fn expr_transformations_nested() {
        let tests = vec![
            // lower + lower
            (
                "lower(lower(kong.foo.foo15)) == \"foo\"",
                "(lower(lower(kong.foo.foo15)) == \"foo\")",
            ),
            // lower + any
            (
                "lower(any(kong.foo.foo16)) == \"foo\"",
                "(lower(any(kong.foo.foo16)) == \"foo\")",
            ),
            // any + lower
            (
                "any(lower(kong.foo.foo17)) == \"foo\"",
                "(any(lower(kong.foo.foo17)) == \"foo\")",
            ),
            // any + any
            (
                "any(any(kong.foo.foo18)) == \"foo\"",
                "(any(any(kong.foo.foo18)) == \"foo\")",
            ),
        ];
        for (input, expected) in tests {
            let result = parse(input).unwrap();
            assert_eq!(result.to_string(), expected);
        }
    }

    #[test]
    fn str_unicode_test() {
        let tests = vec![
            // cjk chars
            ("t_msg in \"你好\"", "(t_msg in \"你好\")"),
            // 0xXXX unicode
            ("t_msg in \"\u{4f60}\u{597d}\"", "(t_msg in \"你好\")"),
        ];
        for (input, expected) in tests {
            let result = parse(input).unwrap();
            assert_eq!(result.to_string(), expected);
        }
    }

    #[test]
    fn rawstr_test() {
        let tests = vec![
            // invalid escape sequence
            (r##"a == r#"/path/to/\d+"#"##, r#"(a == "/path/to/\d+")"#),
            // valid escape sequence
            (r##"a == r#"/path/to/\n+"#"##, r#"(a == "/path/to/\n+")"#),
        ];
        for (input, expected) in tests {
            let result = parse(input).unwrap();
            assert_eq!(result.to_string(), expected);
        }
    }
}
