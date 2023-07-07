#![allow(deprecated)]
extern crate pest;

use crate::ast::{
    BinaryOperator, Expression, Lhs, LhsTransformations, LogicalExpression, Predicate, Value,
};
use cidr::{IpCidr, Ipv4Cidr, Ipv6Cidr};
use pest::error::ErrorVariant;
use pest::prec_climber::{Assoc, Operator, PrecClimber};
use pest_consume::{match_nodes, Error as ParseError, Parser};
use regex::Regex;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

type ParseResult<T> = Result<T, ParseError<Rule>>;
/// cbindgen:ignore
// Bug: https://github.com/eqrion/cbindgen/issues/286
type Node<'i> = pest_consume::Node<'i, Rule, ()>;

trait IntoParseResult<T> {
    fn into_parse_result(self, node: &Node) -> ParseResult<T>;
}

impl<T, E> IntoParseResult<T> for Result<T, E>
where
    E: ToString,
{
    fn into_parse_result(self, node: &Node) -> ParseResult<T> {
        self.map_err(|e| {
            let span = node.as_span();

            let err_var = ErrorVariant::CustomError {
                message: e.to_string(),
            };

            ParseError::new_from_span(err_var, span)
        })
    }
}

#[derive(Parser)]
#[grammar = "atc_grammar.pest"]
struct ATCParser;

lazy_static::lazy_static! {
    static ref PRECCLIMBER: PrecClimber<Rule> = PrecClimber::new(
        vec![
            Operator::new(Rule::and_op, Assoc::Left) | Operator::new(Rule::or_op, Assoc::Left),
        ]
    );
}

macro_rules! parse_num {
    ($node:expr, $ty:ident, $radix:expr) => {
        $ty::from_str_radix($node.as_str(), $radix).into_parse_result(&$node)
    };
}

#[pest_consume::parser]
impl ATCParser {
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn ident(input: Node) -> ParseResult<String> {
        Ok(input.as_str().into())
    }

    fn str_esc(input: Node) -> ParseResult<char> {
        Ok(match input.as_str() {
            "\\\"" => '"',
            "\\\\" => '\\',
            "\\n" => '\n',
            "\\r" => '\r',
            "\\t" => '\t',
            _ => unreachable!(),
        })
    }

    fn str_literal(input: Node) -> ParseResult<String> {
        let mut s = String::new();

        for node in input.into_children() {
            match node.as_rule() {
                Rule::str_char => s.push_str(node.as_str()),
                Rule::str_esc => s.push(ATCParser::str_esc(node)?),
                _ => unreachable!(),
            }
        }

        Ok(s)
    }
    fn rawstr_literal(input: Node) -> ParseResult<String> {
        let mut s = String::new();

        for node in input.into_children() {
            match node.as_rule() {
                Rule::rawstr_char => s.push_str(node.as_str()),
                _ => unreachable!(),
            }
        }

        Ok(s)
    }

    fn ipv4_cidr_literal(input: Node) -> ParseResult<Ipv4Cidr> {
        input.as_str().parse().into_parse_result(&input)
    }

    fn ipv6_cidr_literal(input: Node) -> ParseResult<Ipv6Cidr> {
        input.as_str().parse().into_parse_result(&input)
    }

    fn ipv4_literal(input: Node) -> ParseResult<Ipv4Addr> {
        input.as_str().parse().into_parse_result(&input)
    }

    fn ipv6_literal(input: Node) -> ParseResult<Ipv6Addr> {
        input.as_str().parse().into_parse_result(&input)
    }

    fn int_literal(input: Node) -> ParseResult<i64> {
        use Rule::*;

        let digits_node = input.children().single().unwrap();

        let radix = match digits_node.as_rule() {
            hex_digits => 16,
            oct_digits => 8,
            dec_digits => 10,
            _ => unreachable!(),
        };

        let mut num = parse_num!(digits_node, i64, radix)?;

        if let Some('-') = input.as_str().chars().next() {
            num = -num;
        }

        Ok(num)
    }

    fn rhs(input: Node) -> ParseResult<Value> {
        Ok(match_nodes! { input.children();
            [str_literal(s)] => Value::String(s),
            [rawstr_literal(s)] => Value::String(s),
            [ipv4_cidr_literal(c)] => Value::IpCidr(IpCidr::V4(c)),
            [ipv6_cidr_literal(c)] => Value::IpCidr(IpCidr::V6(c)),
            [ipv4_literal(i)] => Value::IpAddr(IpAddr::V4(i)),
            [ipv6_literal(i)] => Value::IpAddr(IpAddr::V6(i)),
            [int_literal(i)] => Value::Int(i),
        })
    }

    fn transform_func(input: Node) -> ParseResult<Lhs> {
        Ok(match_nodes! { input.children();
            [func_name, lhs(mut lhs)] => {
                lhs.transformations.push(match func_name.as_str() {
                    "lower" => LhsTransformations::Lower,
                    "any" => LhsTransformations::Any,
                    unknown => {
                        return Err(ParseError::new_from_span(
                            ErrorVariant::CustomError {
                                message: format!("unknown transformation function: {}", unknown),
                            },
                            input.as_span()));
                    },
                });

                lhs
            },
        })
    }

    fn lhs(input: Node) -> ParseResult<Lhs> {
        Ok(match_nodes! { input.children();
            [transform_func(t)] => t,
            [ident(var)] => Lhs { var_name: var, transformations: Vec::new() },
        })
    }

    fn binary_operator(input: Node) -> ParseResult<BinaryOperator> {
        use BinaryOperator::*;

        Ok(match input.as_str() {
            "==" => Equals,
            "!=" => NotEquals,
            "~" => Regex,
            "^=" => Prefix,
            "=^" => Postfix,
            ">" => Greater,
            ">=" => GreaterOrEqual,
            "<" => Lesser,
            "<=" => LesserOrEqual,
            "in" => In,
            "contains" => Contains,
            _ => NotIn,
        })
    }

    fn predicate(input: Node) -> ParseResult<Expression> {
        Ok(match_nodes! { input.children();
            [lhs(lhs), binary_operator(op), rhs(rhs)] => {
                Expression::Predicate(Predicate{ lhs,
                    rhs: if op == BinaryOperator::Regex {
                        if let Value::String(s) = rhs {
                            let r = Regex::new(&s)
                                .map_err(|e| ParseError::new_from_span(
                                ErrorVariant::CustomError {
                                    message: e.to_string(),
                                }, input.as_span()))?;

                            Value::Regex(r)
                        } else {
                            return Err(ParseError::new_from_span(
                                ErrorVariant::CustomError {
                                    message: "regex operator can only be used with String operands".to_string(),
                                },
                            input.as_span()));
                        }
                    } else {
                        rhs
                    },
                    op })
            },
        })
    }

    fn parenthesised_expression(input: Node) -> ParseResult<Expression> {
        Ok(match_nodes! { input.children();
            [expression(expr)] => expr,
        })
    }

    #[prec_climb(term, PRECCLIMBER)]
    fn expression(l: Expression, op: Node, r: Expression) -> ParseResult<Expression> {
        Ok(match op.as_rule() {
            Rule::and_op => Expression::Logical(Box::new(LogicalExpression::And(l, r))),
            Rule::or_op => Expression::Logical(Box::new(LogicalExpression::Or(l, r))),
            _ => unreachable!(),
        })
    }

    fn term(input: Node) -> ParseResult<Expression> {
        Ok(match_nodes! { input.children();
            [predicate(expr)] => expr,
            [parenthesised_expression(expr)] => expr,
        })
    }

    fn matcher(input: Node) -> ParseResult<Expression> {
        Ok(match_nodes! { input.children();
            [expression(expr), EOI(_)] => expr,
        })
    }
}

pub fn parse(atc: &str) -> ParseResult<Expression> {
    let matchers = ATCParser::parse(Rule::matcher, atc)?;
    let matcher = matchers.single()?;
    ATCParser::matcher(matcher)
}
