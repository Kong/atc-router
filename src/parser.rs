extern crate pest;

use crate::ast::{
    BinaryOperator, Expression, Lhs, LhsTransformations, LogicalExpression, Predicate, Value,
};
use cidr::{IpCidr, Ipv4Cidr, Ipv6Cidr};
use pest::error::Error as ParseError;
use pest::error::ErrorVariant;
use pest::iterators::Pair;
use pest::pratt_parser::Assoc as AssocNew;
use pest::pratt_parser::{Op, PrattParser};
use pest::Parser;
use regex::Regex;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use memoize::memoize;

type ParseResult<T> = Result<T, ParseError<Rule>>;
/// cbindgen:ignore
// Bug: https://github.com/eqrion/cbindgen/issues/286

trait IntoParseResult<T> {
    fn into_parse_result(self, pair: &Pair<Rule>) -> ParseResult<T>;
}

impl<T, E> IntoParseResult<T> for Result<T, E>
where
    E: ToString,
{
    fn into_parse_result(self, pair: &Pair<Rule>) -> ParseResult<T> {
        self.map_err(|e| {
            let span = pair.as_span();

            let err_var = ErrorVariant::CustomError {
                message: e.to_string(),
            };

            ParseError::new_from_span(err_var, span)
        })
    }
}

#[derive(Parser)]
#[grammar = "atc_grammar.pest"]
struct ATCParser {
    pratt_parser: PrattParser<Rule>,
}

macro_rules! parse_num {
    ($node:expr, $ty:ident, $radix:expr) => {
        $ty::from_str_radix($node.as_str(), $radix).into_parse_result(&$node)
    };
}

impl ATCParser {
    fn new() -> Self {
        Self {
            pratt_parser: PrattParser::new()
                .op(Op::infix(Rule::and_op, AssocNew::Left))
                .op(Op::infix(Rule::or_op, AssocNew::Left)),
        }
    }
    // matcher = { SOI ~ expression ~ EOI }
    fn parse_matcher(&mut self, source: &str) -> ParseResult<Expression> {
        let pairs = ATCParser::parse(Rule::matcher, source)?;
        let expr_pair = pairs.peek().unwrap().into_inner().peek().unwrap();
        let rule = expr_pair.as_rule();
        match rule {
            Rule::expression => parse_expression(expr_pair, &self.pratt_parser),
            _ => unreachable!(),
        }
    }
}

fn parse_ident(pair: Pair<Rule>) -> ParseResult<String> {
    Ok(pair.as_str().into())
}
fn parse_lhs(pair: Pair<Rule>) -> ParseResult<Lhs> {
    let pairs = pair.into_inner();
    let pair = pairs.peek().unwrap();
    let rule = pair.as_rule();
    Ok(match rule {
        Rule::transform_func => parse_transform_func(pair)?,
        Rule::ident => {
            let var = parse_ident(pair)?;
            Lhs {
                var_name: var,
                transformations: Vec::new(),
            }
        }
        _ => unreachable!(),
    })
}
// rhs = { str_literal | ip_literal | int_literal }
fn parse_rhs(pair: Pair<Rule>) -> ParseResult<Value> {
    let pairs = pair.into_inner();
    let pair = pairs.peek().unwrap();
    let rule = pair.as_rule();
    Ok(match rule {
        Rule::str_literal => Value::String(parse_str_literal(pair)?),
        Rule::rawstr_literal => Value::String(parse_rawstr_literal(pair)?),
        Rule::ipv4_cidr_literal => Value::IpCidr(IpCidr::V4(parse_ipv4_cidr_literal(pair)?)),
        Rule::ipv6_cidr_literal => Value::IpCidr(IpCidr::V6(parse_ipv6_cidr_literal(pair)?)),
        Rule::ipv4_literal => Value::IpAddr(IpAddr::V4(parse_ipv4_literal(pair)?)),
        Rule::ipv6_literal => Value::IpAddr(IpAddr::V6(parse_ipv6_literal(pair)?)),
        Rule::int_literal => Value::Int(parse_int_literal(pair)?),
        _ => unreachable!(),
    })
}

// str_literal = ${ "\"" ~ str_inner ~ "\"" }
fn parse_str_literal(pair: Pair<Rule>) -> ParseResult<String> {
    let char_pairs = pair.into_inner();
    let mut s = String::new();
    for char_pair in char_pairs {
        let rule = char_pair.as_rule();
        match rule {
            Rule::str_esc => s.push(parse_str_esc(char_pair)),
            Rule::str_char => s.push(parse_str_char(char_pair)),
            _ => unreachable!(),
        }
    }
    Ok(s)
}

// rawstr_literal = ${ "r#\"" ~ rawstr_char* ~ "\"#" }
// rawstr_char = { !"\"#" ~ ANY }
fn parse_rawstr_literal(pair: Pair<Rule>) -> ParseResult<String> {
    let char_pairs = pair.into_inner();
    let mut s = String::new();
    for char_pair in char_pairs {
        let rule = char_pair.as_rule();
        match rule {
            Rule::rawstr_char => s.push(parse_str_char(char_pair)),
            _ => unreachable!(),
        }
    }
    Ok(s)
}

fn parse_str_esc(pair: Pair<Rule>) -> char {
    match pair.as_str() {
        r#"\""# => '"',
        r#"\\"# => '\\',
        r#"\n"# => '\n',
        r#"\r"# => '\r',
        r#"\t"# => '\t',

        _ => unreachable!(),
    }
}
fn parse_str_char(pair: Pair<Rule>) -> char {
    return pair.as_str().chars().next().unwrap();
}
fn parse_ipv4_cidr_literal(pair: Pair<Rule>) -> ParseResult<Ipv4Cidr> {
    pair.as_str().parse().into_parse_result(&pair)
}
fn parse_ipv6_cidr_literal(pair: Pair<Rule>) -> ParseResult<Ipv6Cidr> {
    pair.as_str().parse().into_parse_result(&pair)
}
fn parse_ipv4_literal(pair: Pair<Rule>) -> ParseResult<Ipv4Addr> {
    pair.as_str().parse().into_parse_result(&pair)
}
fn parse_ipv6_literal(pair: Pair<Rule>) -> ParseResult<Ipv6Addr> {
    pair.as_str().parse().into_parse_result(&pair)
}

fn parse_int_literal(pair: Pair<Rule>) -> ParseResult<i64> {
    let is_neg = pair.as_str().starts_with('-');
    let pairs = pair.into_inner();
    let pair = pairs.peek().unwrap(); // digits
    let rule = pair.as_rule();
    let radix = match rule {
        Rule::hex_digits => 16,
        Rule::oct_digits => 8,
        Rule::dec_digits => 10,
        _ => unreachable!(),
    };

    let mut num = parse_num!(pair, i64, radix)?;

    if is_neg {
        num = -num;
    }

    Ok(num)
}

#[memoize(Capacity: 1000000)]
pub fn parse_regex(s: String) -> Result<Regex, regex::Error> {
    Regex::new(s.as_str())
}

#[test]
fn expr_regex_memoization() {
    let mut count = 0u32;
    let now = std::time::Instant::now();
    loop {
        parse_regex("^foo.*$".to_string()).unwrap();
        count += 1;
        if count == 50000 {
            break;
        }
    }
    assert!(now.elapsed().as_secs() < 1);
}

// predicate = { lhs ~ binary_operator ~ rhs }
fn parse_predicate(pair: Pair<Rule>) -> ParseResult<Predicate> {
    let mut pairs = pair.into_inner();
    let lhs = parse_lhs(pairs.next().unwrap())?;
    let op = parse_binary_operator(pairs.next().unwrap());
    let rhs_pair = pairs.next().unwrap();
    let rhs = parse_rhs(rhs_pair.clone())?;
    Ok(Predicate {
        lhs,
        rhs: if op == BinaryOperator::Regex {
            if let Value::String(s) = rhs {
                let r = parse_regex(s).map_err(|e| {
                    ParseError::new_from_span(
                        ErrorVariant::CustomError {
                            message: e.to_string(),
                        },
                        rhs_pair.as_span(),
                    )
                })?;

                Value::Regex(r)
            } else {
                return Err(ParseError::new_from_span(
                    ErrorVariant::CustomError {
                        message: "regex operator can only be used with String operands".to_string(),
                    },
                    rhs_pair.as_span(),
                ));
            }
        } else {
            rhs
        },
        op,
    })
}
// transform_func = { ident ~ "(" ~ lhs ~ ")" }
fn parse_transform_func(pair: Pair<Rule>) -> ParseResult<Lhs> {
    let span = pair.as_span();
    let pairs = pair.into_inner();
    let mut pairs = pairs.peekable();
    let func_name = pairs.next().unwrap().as_str().to_string();
    let mut lhs = parse_lhs(pairs.next().unwrap())?;
    lhs.transformations.push(match func_name.as_str() {
        "lower" => LhsTransformations::Lower,
        "any" => LhsTransformations::Any,
        unknown => {
            return Err(ParseError::new_from_span(
                ErrorVariant::CustomError {
                    message: format!("unknown transformation function: {}", unknown),
                },
                span,
            ));
        }
    });

    Ok(lhs)
}
// binary_operator = { "==" | "!=" | "~" | "^=" | "=^" | ">=" |
//                     ">" | "<=" | "<" | "in" | "not" ~ "in" | "contains" }
fn parse_binary_operator(pair: Pair<Rule>) -> BinaryOperator {
    let rule = pair.as_str();
    use BinaryOperator as BinaryOp;
    match rule {
        "==" => BinaryOp::Equals,
        "!=" => BinaryOp::NotEquals,
        "~" => BinaryOp::Regex,
        "^=" => BinaryOp::Prefix,
        "=^" => BinaryOp::Postfix,
        ">=" => BinaryOp::GreaterOrEqual,
        ">" => BinaryOp::Greater,
        "<=" => BinaryOp::LessOrEqual,
        "<" => BinaryOp::Less,
        "in" => BinaryOp::In,
        "not in" => BinaryOp::NotIn,
        "contains" => BinaryOp::Contains,
        _ => unreachable!(),
    }
}

// parenthesised_expression = { "(" ~ expression ~ ")" }
fn parse_parenthesised_expression(
    pair: Pair<Rule>,
    pratt: &PrattParser<Rule>,
) -> ParseResult<Expression> {
    let pairs = pair.into_inner();
    let pair = pairs.peek().unwrap();
    let rule = pair.as_rule();
    match rule {
        Rule::expression => parse_expression(pair, pratt),
        _ => unreachable!(),
    }
}

// term = { predicate | parenthesised_expression }
fn parse_term(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> ParseResult<Expression> {
    let pairs = pair.into_inner();
    let inner_rule = pairs.peek().unwrap();
    let rule = inner_rule.as_rule();
    match rule {
        Rule::predicate => Ok(Expression::Predicate(parse_predicate(inner_rule)?)),
        Rule::parenthesised_expression => parse_parenthesised_expression(inner_rule, pratt),
        _ => unreachable!(),
    }
}

// expression = { term ~ ( logical_operator ~ term )* }
fn parse_expression(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> ParseResult<Expression> {
    let pairs = pair.into_inner();
    pratt
        .map_primary(|operand| match operand.as_rule() {
            Rule::term => parse_term(operand, pratt),
            _ => unreachable!(),
        })
        .map_infix(|lhs, op, rhs| {
            Ok(match op.as_rule() {
                Rule::and_op => Expression::Logical(Box::new(LogicalExpression::And(lhs?, rhs?))),
                Rule::or_op => Expression::Logical(Box::new(LogicalExpression::Or(lhs?, rhs?))),
                _ => unreachable!(),
            })
        })
        .parse(pairs)
}

pub fn parse(source: &str) -> ParseResult<Expression> {
    ATCParser::new().parse_matcher(source)
}
