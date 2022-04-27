mod ast;
extern crate pest;

use crate::ast::{
    BinaryOperator, Expression, LHSTransformations, LogicalExpression, Predicate, LHS, RHS,
};
use cidr::{IpCidr, Ipv4Cidr, Ipv6Cidr};
use pest::error::ErrorVariant;
use pest::prec_climber::{Assoc, Operator, PrecClimber};
use pest_consume::{match_nodes, Error as ParseError, Parser};
use wasm_bindgen::prelude::wasm_bindgen;

type ParseResult<T> = Result<T, ParseError<Rule>>;
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

    fn str_literal(input: Node) -> ParseResult<String> {
        Ok(input.into_children().single()?.as_str().into())
    }

    fn ipv4_cidr_literal(input: Node) -> ParseResult<Ipv4Cidr> {
        input.as_str().parse().into_parse_result(&input)
    }

    fn ipv6_cidr_literal(input: Node) -> ParseResult<Ipv6Cidr> {
        input.as_str().parse().into_parse_result(&input)
    }

    fn ipv4_literal(input: Node) -> ParseResult<Ipv4Cidr> {
        format!("{}/32", input.as_str())
            .parse()
            .into_parse_result(&input)
    }

    fn ipv6_literal(input: Node) -> ParseResult<Ipv6Cidr> {
        format!("{}/128", input.as_str())
            .parse()
            .into_parse_result(&input)
    }

    fn ip_literal(input: Node) -> ParseResult<IpCidr> {
        Ok(match_nodes! { input.children();
            [ipv4_cidr_literal(c)] => IpCidr::V4(c),
            [ipv6_cidr_literal(c)] => IpCidr::V6(c),
            [ipv4_literal(c)] => IpCidr::V4(c),
            [ipv6_literal(c)] => IpCidr::V6(c),
        })
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

    fn rhs(input: Node) -> ParseResult<RHS> {
        Ok(match_nodes! { input.children();
            [str_literal(s)] => RHS::String(s),
            [ip_literal(ip)] => RHS::IpCidr(ip),
            [int_literal(i)] => RHS::Int(i),
        })
    }

    fn transform_func(input: Node) -> ParseResult<LHS> {
        let mut iter = input.children();
        let func_name = iter.next().unwrap();
        let var_name = iter.next().unwrap();
        // currently only "lower()" is supported from grammar
        assert_eq!(func_name.as_str(), "lower");

        Ok(LHS {
            var_name: var_name.as_str().into(),
            transformation: Some(LHSTransformations::Lower),
        })
    }

    fn lhs(input: Node) -> ParseResult<LHS> {
        Ok(match_nodes! { input.children();
            [transform_func(t)] => t,
            [ident(var)] => LHS { var_name: var, transformation: None },
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
            _ => {
                let mut iter = input.into_children();
                assert_eq!("not", iter.next().unwrap().as_str());
                assert_eq!("in", iter.next().unwrap().as_str());
                NotIn
            }
        })
    }

    fn predicate(input: Node) -> ParseResult<Expression> {
        Ok(match_nodes! { input.children();
            [lhs(lhs), binary_operator(op), rhs(rhs)] => Expression::Predicate(Predicate{lhs, rhs, op}),
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

#[wasm_bindgen]
pub fn parse(atc: &str) -> Result<String, String> {
    match ATCParser::parse(Rule::matcher, atc) {
        Ok(matcher) => Ok(serde_json::to_string(
            &ATCParser::matcher(matcher.single().unwrap()).unwrap(),
        )
        .unwrap()),
        Err(e) => Err(e.to_string()),
    }
}
