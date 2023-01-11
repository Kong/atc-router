use std::fmt;

use crate::ast::*;

#[cfg(test)]
use crate::parser::parse;

impl Expression {
    pub fn to_string(&self) -> String {
        match self {
            Expression::Logical(logical) => logical.to_string(),
            Expression::Predicate(predicate) => predicate.to_string(),
        }
    }
}
impl LogicalExpression {
    pub fn to_string(&self) -> String {
        match self {
            LogicalExpression::And(left, right) => {
                format!("({} && {})", left.to_string(), right.to_string())
            }
            LogicalExpression::Or(left, right) => {
                format!("({} || {})", left.to_string(), right.to_string())
            }
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expression::Logical(logical) => write!(f, "{}", logical.to_string()),
            Expression::Predicate(predicate) => write!(f, "{}", predicate.to_string()),
        }
    }
}

impl LhsTransformations {
    pub fn to_string(&self) -> String {
        match self {
            LhsTransformations::Lower => "lower".to_string(),
            LhsTransformations::Any => "any".to_string(),
        }
    }
}
impl BinaryOperator {
    pub fn to_string(&self) -> String {
        use BinaryOperator::*;
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
        }
        .to_string()
    }
}

impl Value {
    pub fn to_string(&self) -> String {
        match self {
            Value::String(s) => format!("\"{}\"", s),
            Value::IpCidr(cidr) => cidr.to_string(),
            Value::IpAddr(addr) => addr.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Regex(re) => re.to_string(),
        }
    }
}

impl Lhs {
    pub fn to_string(&self) -> String {
        let mut s = self.var_name.to_string();
        for transformation in &self.transformations {
            s = format!("{}({})", transformation.to_string(), s);
        }
        s
    }
}
impl Predicate {
    pub fn to_string(&self) -> String {
        format!(
            "({} {} {})",
            self.lhs.to_string(),
            self.op.to_string(),
            self.rhs.to_string()
        )
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
