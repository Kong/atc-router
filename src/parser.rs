use crate::ast;
use crate::ast::BinaryOperator;
use cidr::IpCidr;
use regex::Regex;
use std::borrow::Cow;
use std::net::IpAddr;
use winnow::ascii::{digit1, escaped, hex_digit1, oct_digit0};
use winnow::combinator::Infix::Left;
use winnow::combinator::{
    alt, backtrack_err, cut_err, delimited, eof, fail, opt, peek, preceded, repeat, separated,
    separated_pair, terminated, trace,
};
use winnow::dispatch;
use winnow::error::{AddContext, ContextError, ParserError, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::stream::AsChar;
use winnow::token::{any, one_of, take, take_until, take_while};

fn expect_char<'a, E: ParserError<&'a str>>(ch: char) -> impl Parser<&'a str, char, E>
where
    E: AddContext<&'a str, StrContext>,
{
    ch.context(StrContext::Expected(StrContextValue::CharLiteral(ch)))
}

fn whitespace<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> Result<&'a str, E> {
    trace("whitespace", take_while(0.., (' ', '\t', '\r', '\n'))).parse_next(input)
}

fn ws<'a, F, O, E: ParserError<&'a str>>(inner: F) -> impl Parser<&'a str, O, E>
where
    F: Parser<&'a str, O, E>,
{
    delimited(whitespace, inner, whitespace)
}

fn ident<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    (
        one_of(|c: char| c.is_ascii_alphabetic()),
        take_while(0.., |c: char| {
            c.is_ascii_alphanumeric() || c == '_' || c == '.'
        }),
    )
        .take()
        .parse_next(input)
}

fn int_literal(input: &mut &str) -> ModalResult<i64> {
    fn inner(input: &mut &str) -> ModalResult<i64> {
        let sign = opt("-").parse_next(input)?;

        let hex = preceded("0x", cut_err(hex_digit1))
            .try_map(|s| i64::from_str_radix(s, 16))
            .context(StrContext::Label("hexadecimal integer"));

        // TODO: the pest grammar accepts `09` as decimal (but fails with 009), because it
        //       only requires at least one octal digit after `0` before it
        //       commits to parsing as octal
        let octal = preceded(
            "0",
            (one_of(AsChar::is_oct_digit), cut_err(oct_digit0)).take(),
        )
        .try_map(|s: &str| i64::from_str_radix(s, 8))
        .context(StrContext::Label("octal integer"));
        let decimal = digit1
            .parse_to()
            .context(StrContext::Label("decimal integer"));

        let value = alt((hex, octal, decimal)).parse_next(input)?;

        let value = match sign {
            Some(_) => -value,
            None => value,
        };

        Ok(value)
    }
    inner.context(StrContext::Label("int")).parse_next(input)
}

fn str_literal(input: &mut &str) -> ModalResult<String> {
    delimited(
        '"',
        cut_err(escaped(
            take_until(1.., ('"', '\\')),
            '\\',
            alt((
                '\\'.value("\\"),
                '"'.value("\""),
                'n'.value("\n"),
                'r'.value("\r"),
                't'.value("\t"),
                fail.context(StrContext::Label("escape sequence"))
                    .context_with(|| {
                        ['\\', '"', 'n', 'r', 't']
                            .into_iter()
                            .map(|ch| StrContext::Expected(StrContextValue::CharLiteral(ch)))
                    }),
            )),
        )),
        cut_err(expect_char('"')),
    )
    .context(StrContext::Label("string"))
    .parse_next(input)
}

fn rawstr_literal<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    // TODO: arbitrary number of #
    delimited("r#\"", cut_err(take_until(0.., "\"#")), cut_err("\"#"))
        .context(StrContext::Label("raw string"))
        .parse_next(input)
}

fn anystr_literal<'a>(input: &mut &'a str) -> ModalResult<Cow<'a, str>> {
    dispatch!(peek(any);
        '"' => cut_err(str_literal.map(Cow::Owned)),
        'r' => cut_err(rawstr_literal.map(Cow::Borrowed)),
        _ => fail.context(StrContext::Label("string")),
    )
    .parse_next(input)
}

fn ipv4_literal(input: &mut &str) -> ModalResult<std::net::Ipv4Addr> {
    let octet = || take_while(1..=3, AsChar::is_dec_digit);

    (
        octet(),
        '.',
        cut_err(separated(3, octet(), '.').map(|()| ())),
    )
        .take()
        .parse_to()
        .context(StrContext::Label("ipv4 address"))
        .parse_next(input)
}

fn ipv6_literal(input: &mut &str) -> ModalResult<std::net::Ipv6Addr> {
    // TODO: ipv4 mapped addresses
    let segment = || take_while(1..=4, |c: char| c.is_ascii_hexdigit()).void();
    (
        alt((':'.void(), segment())),
        ':',
        cut_err(repeat(0.., alt((':'.void(), segment()))).map(|()| ())),
    )
        .take()
        .parse_to()
        .context(StrContext::Label("ipv6 address"))
        .parse_next(input)
}

fn ipv4_cidr_literal(input: &mut &str) -> ModalResult<IpCidr> {
    separated_pair(
        backtrack_err(ipv4_literal.map(IpAddr::V4)),
        expect_char('/'),
        cut_err(take_while(1..=2, |c: char| c.is_ascii_digit()).parse_to()),
    )
    .try_map(|(addr, prefix)| IpCidr::new(addr, prefix))
    .context(StrContext::Label("ipv4 cidr"))
    .parse_next(input)
}

fn ipv6_cidr_literal(input: &mut &str) -> ModalResult<IpCidr> {
    separated_pair(
        backtrack_err(ipv6_literal.map(IpAddr::V6)),
        expect_char('/'),
        cut_err(take_while(1..=3, |c: char| c.is_ascii_digit()).parse_to()),
    )
    .try_map(|(addr, prefix)| IpCidr::new(addr, prefix))
    .context(StrContext::Label("ipv6 cidr"))
    .parse_next(input)
}

fn rhs(input: &mut &str) -> ModalResult<ast::Value> {
    alt((
        anystr_literal.map(Cow::into_owned).map(ast::Value::String),
        ipv4_cidr_literal.map(ast::Value::IpCidr),
        ipv6_cidr_literal.map(ast::Value::IpCidr),
        ipv4_literal.map(IpAddr::V4).map(ast::Value::IpAddr),
        ipv6_literal.map(IpAddr::V6).map(ast::Value::IpAddr),
        int_literal.map(ast::Value::Int),
    ))
    .parse_next(input)
}

fn lhs(input: &mut &str) -> ModalResult<ast::Lhs> {
    alt((
        transform_func,
        ident.map(|ident| ast::Lhs {
            var_name: ident.to_string(),
            transformations: vec![],
        }),
    ))
    .context(StrContext::Label("lhs"))
    .parse_next(input)
}

fn transform_func(input: &mut &str) -> ModalResult<ast::Lhs> {
    let start = input.checkpoint();
    let func_name = ident.parse_next(input)?;
    _ = ws('(').parse_next(input)?;

    // We found a transform function call for sure
    let mut lhs_val = cut_err(terminated(
        lhs,
        ws(')'.context(StrContext::Expected(StrContextValue::CharLiteral(')')))),
    ))
    .parse_next(input)?;

    let transform = match func_name {
        "lower" => ast::LhsTransformations::Lower,
        "any" => ast::LhsTransformations::Any,
        _ => {
            input.reset(&start);
            return cut_err(fail)
                .context(StrContext::Label("transform function"))
                .context_with(|| {
                    ["lower", "any"]
                        .into_iter()
                        .map(|s| StrContext::Expected(StrContextValue::StringLiteral(s)))
                })
                .parse_next(input);
        }
    };

    lhs_val.transformations.push(transform);
    Ok(lhs_val)
}

fn not_in<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    ("not", whitespace, "in").take().parse_next(input)
}

fn binary_operator(input: &mut &str) -> ModalResult<ast::BinaryOperator> {
    use ast::BinaryOperator::*;
    alt((
        "==".value(Equals),
        "!=".value(NotEquals),
        "~".value(Regex),
        "^=".value(Prefix),
        "=^".value(Postfix),
        ">=".value(GreaterOrEqual),
        ">".value(Greater),
        "<=".value(LessOrEqual),
        "<".value(Less),
        not_in.value(NotIn),
        "in".value(In),
        "contains".value(Contains),
        fail.context(StrContext::Label("binary operator"))
            .context_with(|| {
                [
                    "==", "!=", "~", "^=", "=^", ">=", ">", "<=", "<", "not in", "in", "contains",
                ]
                .into_iter()
                .map(|s| StrContext::Expected(StrContextValue::StringLiteral(s)))
            }),
    ))
    .parse_next(input)
}

fn predicate(input: &mut &str) -> ModalResult<ast::Predicate> {
    fn inner(input: &mut &str) -> ModalResult<ast::Predicate> {
        let (lhs_value, binary_op) =
            separated_pair(lhs, whitespace, binary_operator).parse_next(input)?;
        let _ = whitespace(input)?;
        // TODO: original parser doesn't validate types with other operators, but we could e.g.
        //       directly try to only parse an int for `>`
        let rhs_value = if binary_op == BinaryOperator::Regex {
            ast::Value::Regex(
                anystr_literal
                    .try_map(|s| Regex::new(&s))
                    .context(StrContext::Label("regex"))
                    .parse_next(input)?,
            )
        } else {
            rhs.parse_next(input)?
        };
        /*
        let rhs_value = match binary_op {
            BinaryOperator::Equals | BinaryOperator::NotEquals => rhs.parse_next(input)?,
            BinaryOperator::Regex => ast::Value::Regex(
                anystr_literal
                    .try_map(|s| Regex::new(&s))
                    .context(StrContext::Label("regex literal"))
                    .parse_next(input)?,
            ),
            BinaryOperator::Prefix | BinaryOperator::Postfix | BinaryOperator::Contains => {
                let rhs_str = anystr_literal.parse_next(input)?;
                ast::Value::String(rhs_str.into_owned())
            }
            BinaryOperator::Greater
            | BinaryOperator::GreaterOrEqual
            | BinaryOperator::Less
            | BinaryOperator::LessOrEqual => ast::Value::Int(int_literal.parse_next(input)?),
            BinaryOperator::In | BinaryOperator::NotIn => alt((
                ipv4_cidr_literal.map(ast::Value::IpCidr),
                ipv6_cidr_literal.map(ast::Value::IpCidr),
            ))
            .parse_next(input)?,
        };
         */
        Ok(ast::Predicate {
            lhs: lhs_value,
            op: binary_op,
            rhs: rhs_value,
        })
    }

    cut_err(inner)
        .context(StrContext::Label("predicate"))
        .parse_next(input)
}

fn parenthesised_expression(input: &mut &str) -> ModalResult<ast::Expression> {
    (
        opt('!').map(|opt| opt.is_some()),
        whitespace,
        cut_err(delimited(
            '('.context(StrContext::Expected(StrContextValue::CharLiteral('('))),
            ws(expression),
            ')'.context(StrContext::Expected(StrContextValue::CharLiteral(')'))),
        )),
    )
        .map(|(invert, _, expr)| {
            if invert {
                ast::Expression::Logical(Box::new(ast::LogicalExpression::Not(expr)))
            } else {
                expr
            }
        })
        .context(StrContext::Label("parenthesised expression"))
        .parse_next(input)
}

fn term(input: &mut &str) -> ModalResult<ast::Expression> {
    dispatch!(peek(any);
        '!' | '(' => parenthesised_expression,
        _ => predicate.map(ast::Expression::Predicate),
    )
    .context(StrContext::Label("term"))
    .parse_next(input)
}
fn expression(input: &mut &str) -> ModalResult<ast::Expression> {
    winnow::combinator::expression(ws(term))
        .infix(dispatch!(take(2usize);
            "&&" => Left(1, |_, lhs, rhs| Ok(ast::Expression::Logical(Box::new(ast::LogicalExpression::And(lhs, rhs))))),
            "||" => Left(2, |_, lhs, rhs| Ok(ast::Expression::Logical(Box::new(ast::LogicalExpression::Or(lhs, rhs))))),
            _ => fail,
        ))
        .context(StrContext::Label("expression"))
        .parse_next(input)
}

pub fn parse(
    input: &str,
) -> Result<ast::Expression, winnow::error::ParseError<&str, ContextError>> {
    terminated(
        expression,
        eof.context(StrContext::Expected(StrContextValue::Description("eof")))
            .context(StrContext::Expected(StrContextValue::StringLiteral("&&")))
            .context(StrContext::Expected(StrContextValue::StringLiteral("||"))),
    )
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bad_syntax() {
        insta::assert_snapshot!(
            parse("! a == 1").unwrap_err(),
            @"
        ! a == 1
          ^
        invalid parenthesised expression
        expected `(`
        "
        );
        insta::assert_snapshot!(
            parse("a == 1 || ! b == 2").unwrap_err(),
            @"
        a == 1 || ! b == 2
                    ^
        invalid parenthesised expression
        expected `(`
        "
        );
        insta::assert_snapshot!(
            parse("(a == 1 || b == 2) && ! c == 3")
                .unwrap_err(),
            @"
        (a == 1 || b == 2) && ! c == 3
                                ^
        invalid parenthesised expression
        expected `(`
        "
        );

        insta::assert_snapshot!(
            parse(r##"a ~ "[""##).unwrap_err(),
            @r#"
        a ~ "["
            ^
        invalid regex
        regex parse error:
            [
            ^
        error: unclosed character class
        "#
        );
    }

    #[test]
    fn unclosed_parens() {
        insta::assert_snapshot!(
        parse(r#"lower(abc"#).unwrap_err(),
        @"
        lower(abc
                 ^
        invalid lhs
        expected `)`
        "
        );
    }

    #[test]
    fn trailing_garbage() {
        insta::assert_snapshot!(
            parse(r#"a == 1 garbage"#).unwrap_err(),
            @"
        a == 1 garbage
               ^
        expected eof, `&&`, `||`
        "
        )
    }
}
