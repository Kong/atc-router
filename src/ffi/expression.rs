use crate::ast::{BinaryOperator, Expression, LogicalExpression, Predicate};
use crate::ffi::ERR_BUF_MAX_LEN;
use crate::schema::Schema;
use bitflags::bitflags;
use std::cmp::min;
use std::ffi;
use std::os::raw::c_char;
use std::slice::from_raw_parts_mut;

use std::iter::Iterator;

struct PredicateIterator<'a> {
    stack: Vec<&'a Expression>,
}

impl<'a> PredicateIterator<'a> {
    fn new(expr: &'a Expression) -> Self {
        Self { stack: vec![expr] }
    }
}

impl<'a> Iterator for PredicateIterator<'a> {
    type Item = &'a Predicate;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(expr) = self.stack.pop() {
            match expr {
                Expression::Logical(l) => match l.as_ref() {
                    LogicalExpression::And(l, r) | LogicalExpression::Or(l, r) => {
                        self.stack.push(l);
                        self.stack.push(r);
                    }
                    LogicalExpression::Not(r) => {
                        self.stack.push(r);
                    }
                },
                Expression::Predicate(p) => return Some(p),
            }
        }
        None
    }
}

impl Expression {
    fn iter_predicates(&self) -> PredicateIterator {
        PredicateIterator::new(self)
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(C)]
    pub struct BinaryOperatorFlags: u64 /* We can only have no more than 64 BinaryOperators */ {
        const EQUALS = 1 << 0;
        const NOT_EQUALS = 1 << 1;
        const REGEX = 1 << 2;
        const PREFIX = 1 << 3;
        const POSTFIX = 1 << 4;
        const GREATER = 1 << 5;
        const GREATER_OR_EQUAL = 1 << 6;
        const LESS = 1 << 7;
        const LESS_OR_EQUAL = 1 << 8;
        const IN = 1 << 9;
        const NOT_IN = 1 << 10;
        const CONTAINS = 1 << 11;

        const UNUSED = !(Self::EQUALS.bits()
            | Self::NOT_EQUALS.bits()
            | Self::REGEX.bits()
            | Self::PREFIX.bits()
            | Self::POSTFIX.bits()
            | Self::GREATER.bits()
            | Self::GREATER_OR_EQUAL.bits()
            | Self::LESS.bits()
            | Self::LESS_OR_EQUAL.bits()
            | Self::IN.bits()
            | Self::NOT_IN.bits()
            | Self::CONTAINS.bits());
    }
}

impl From<&BinaryOperator> for BinaryOperatorFlags {
    fn from(op: &BinaryOperator) -> Self {
        match op {
            BinaryOperator::Equals => Self::EQUALS,
            BinaryOperator::NotEquals => Self::NOT_EQUALS,
            BinaryOperator::Regex => Self::REGEX,
            BinaryOperator::Prefix => Self::PREFIX,
            BinaryOperator::Postfix => Self::POSTFIX,
            BinaryOperator::Greater => Self::GREATER,
            BinaryOperator::GreaterOrEqual => Self::GREATER_OR_EQUAL,
            BinaryOperator::Less => Self::LESS,
            BinaryOperator::LessOrEqual => Self::LESS_OR_EQUAL,
            BinaryOperator::In => Self::IN,
            BinaryOperator::NotIn => Self::NOT_IN,
            BinaryOperator::Contains => Self::CONTAINS,
        }
    }
}

pub const ATC_ROUTER_EXPRESSION_VALIDATE_OK: i64 = 0;
pub const ATC_ROUTER_EXPRESSION_VALIDATE_FAILED: i64 = 1;
pub const ATC_ROUTER_EXPRESSION_VALIDATE_BUF_TOO_SMALL: i64 = 2;

/// Validates an ATC expression against a schema and get its elements.
///
/// # Arguments
///
/// - `atc`: a C-style string representing the ATC expression.
/// - `schema`: a valid pointer to a [`Schema`] object, as returned by [`schema_new`].
/// - `fields_buf`: a buffer for storing the fields used in the expression.
/// - `fields_buf_len`: a pointer to the length of `fields_buf`.
/// - `fields_total`: a pointer for storing the total number of unique fields used in the expression.
/// - `operators`: a pointer for storing the bitflags representing used operators.
/// - `errbuf`: a buffer to store any error messages.
/// - `errbuf_len`: a pointer to the length of the error message buffer.
///
/// # Returns
///
/// An integer indicating the validation result:
/// - `ATC_ROUTER_EXPRESSION_VALIDATE_OK` (0): Validation succeeded.
/// - `ATC_ROUTER_EXPRESSION_VALIDATE_FAILED` (1): Validation failed; `errbuf` and `errbuf_len` will be updated with an error message.
/// - `ATC_ROUTER_EXPRESSION_VALIDATE_BUF_TOO_SMALL` (2): The provided `fields_buf` is too small.
///
/// If `fields_buf_len` indicates that `fields_buf` is sufficient, this function writes the used fields to `fields_buf`, each field terminated by `\0`.
/// It updates `fields_buf_len` with the required buffer length and stores the total number of fields in `fields_total`.
///
/// If `fields_buf_len` indicates that `fields_buf` is insufficient, it returns `ATC_ROUTER_EXPRESSION_VALIDATE_BUF_TOO_SMALL`.
///
/// It writes the used operators as bitflags to `operators`.
/// Bitflags are defined by `BinaryOperatorFlags` and must exclude bits from `BinaryOperatorFlags::UNUSED`.
///
///
/// # Safety
///
/// Violating any of the following constraints results in undefined behavior:
///
/// - `atc` must be a valid pointer to a C-style string, properly aligned, and must not contain an internal `\0`.
/// - `schema` must be a valid pointer returned by [`schema_new`].
/// - `fields_buf`, must be valid for writing `fields_buf_len * size_of::<u8>()` bytes and properly aligned.
/// - `fields_buf_len` must be a valid pointer to write `size_of::<usize>()` bytes and properly aligned.
/// - `fields_total` must be a valid pointer to write `size_of::<usize>()` bytes and properly aligned.
/// - `operators` must be a valid pointer to write `size_of::<u64>()` bytes and properly aligned.
/// - `errbuf` must be valid for reading and writing `errbuf_len * size_of::<u8>()` bytes and properly aligned.
/// - `errbuf_len` must be a valid pointer for reading and writing `size_of::<usize>()` bytes and properly aligned.

#[no_mangle]
pub unsafe extern "C" fn expression_validate(
    atc: *const u8,
    schema: &Schema,
    fields_buf: *mut u8,
    fields_buf_len: *mut usize,
    fields_total: *mut usize,
    operators: *mut u64,
    errbuf: *mut u8,
    errbuf_len: *mut usize,
) -> i64 {
    use std::collections::HashSet;

    use crate::parser::parse;
    use crate::semantics::Validate;

    let atc = ffi::CStr::from_ptr(atc as *const c_char).to_str().unwrap();
    let errbuf = from_raw_parts_mut(errbuf, ERR_BUF_MAX_LEN);

    // Parse the expression
    let result = parse(atc).map_err(|e| e.to_string());
    if let Err(e) = result {
        let errlen = min(e.len(), *errbuf_len);
        errbuf[..errlen].copy_from_slice(&e.as_bytes()[..errlen]);
        *errbuf_len = errlen;
        return ATC_ROUTER_EXPRESSION_VALIDATE_FAILED;
    }
    // Unwrap is safe since we've already checked for error
    let ast = result.unwrap();

    // Validate expression with schema
    if let Err(e) = ast.validate(schema).map_err(|e| e.to_string()) {
        let errlen = min(e.len(), *errbuf_len);
        errbuf[..errlen].copy_from_slice(&e.as_bytes()[..errlen]);
        *errbuf_len = errlen;
        return ATC_ROUTER_EXPRESSION_VALIDATE_FAILED;
    }

    // Iterate over predicates to get fields and operators
    let mut ops = BinaryOperatorFlags::empty();
    let mut existed_fields = HashSet::new();
    let mut total_fields_length = 0;
    let mut fields_buf_ptr = fields_buf;
    *fields_total = 0;

    for pred in ast.iter_predicates() {
        ops |= BinaryOperatorFlags::from(&pred.op);

        let field = pred.lhs.var_name.as_str();

        if existed_fields.insert(field) {
            // Fields is not existed yet.
            let field = ffi::CString::new(field).unwrap();
            let field_slice = field.as_bytes_with_nul();
            let field_len = field_slice.len();

            *fields_total += 1;
            total_fields_length += field_len;

            if *fields_buf_len < total_fields_length {
                return ATC_ROUTER_EXPRESSION_VALIDATE_BUF_TOO_SMALL;
            }

            let fields_buf = from_raw_parts_mut(fields_buf_ptr, field_len);
            fields_buf.copy_from_slice(field_slice);
            fields_buf_ptr = fields_buf_ptr.add(field_len);
        }
    }

    *fields_buf_len = total_fields_length;
    *operators = ops.bits();

    ATC_ROUTER_EXPRESSION_VALIDATE_OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Type;

    fn expr_validate_on(
        schema: &Schema,
        atc: &str,
        fields_buf_size: usize,
    ) -> Result<(Vec<String>, usize, u64), (i64, String)> {
        let atc = ffi::CString::new(atc).unwrap();
        let mut errbuf = vec![b'X'; ERR_BUF_MAX_LEN];
        let mut errbuf_len = ERR_BUF_MAX_LEN;

        let mut fields_buf = vec![0u8; fields_buf_size];
        let mut fields_buf_len = fields_buf.len();
        let mut fields_total = 0;
        let mut operators = 0u64;

        let result = unsafe {
            expression_validate(
                atc.as_bytes().as_ptr(),
                &schema,
                fields_buf.as_mut_ptr(),
                &mut fields_buf_len,
                &mut fields_total,
                &mut operators,
                errbuf.as_mut_ptr(),
                &mut errbuf_len,
            )
        };

        match result {
            ATC_ROUTER_EXPRESSION_VALIDATE_OK => {
                let mut fields = Vec::<String>::with_capacity(fields_total);
                let mut p = 0;
                for _ in 0..fields_total {
                    let field = unsafe { ffi::CStr::from_ptr(fields_buf[p..].as_ptr().cast()) };
                    let len = field.to_bytes().len() + 1;
                    fields.push(field.to_string_lossy().to_string());
                    p += len;
                }
                assert_eq!(fields_buf_len, p, "Fields buffer length mismatch");
                fields.sort();
                Ok((fields, fields_buf_len, operators))
            }
            ATC_ROUTER_EXPRESSION_VALIDATE_FAILED => {
                let err = String::from_utf8(errbuf[..errbuf_len].to_vec()).unwrap();
                Err((result, err))
            }
            ATC_ROUTER_EXPRESSION_VALIDATE_BUF_TOO_SMALL => Err((result, String::new())),
            _ => panic!("Unknown error code"),
        }
    }

    #[test]
    fn test_expression_validate_success() {
        let atc = r##"net.protocol ~ "^https?$" && net.dst.port == 80 && (net.src.ip not in 10.0.0.0/16 || net.src.ip in 10.0.1.0/24) && http.path contains "hello""##;

        let mut schema = Schema::default();
        schema.add_field("net.protocol", Type::String);
        schema.add_field("net.dst.port", Type::Int);
        schema.add_field("net.src.ip", Type::IpAddr);
        schema.add_field("http.path", Type::String);

        let result = expr_validate_on(&schema, atc, 47);

        assert!(result.is_ok(), "Validation failed");
        let (fields, fields_buf_len, ops) = result.unwrap(); // Unwrap is safe since we've already asserted it
        assert_eq!(
            ops,
            (BinaryOperatorFlags::EQUALS
                | BinaryOperatorFlags::REGEX
                | BinaryOperatorFlags::IN
                | BinaryOperatorFlags::NOT_IN
                | BinaryOperatorFlags::CONTAINS)
                .bits(),
            "Operators mismatch"
        );
        assert_eq!(
            fields,
            vec![
                "http.path".to_string(),
                "net.dst.port".to_string(),
                "net.protocol".to_string(),
                "net.src.ip".to_string()
            ],
            "Fields mismatch"
        );
        assert_eq!(fields_buf_len, 47, "Fields buffer length mismatch");
    }

    #[test]
    fn test_expression_validate_failed_parse() {
        let atc = r##"net.protocol ~ "^https?$" && net.dst.port == 80 && (net.src.ip not in 10.0.0.0/16 || net.src.ip in 10.0.1.0) && http.path contains "hello""##;

        let mut schema = Schema::default();
        schema.add_field("net.protocol", Type::String);
        schema.add_field("net.dst.port", Type::Int);
        schema.add_field("net.src.ip", Type::IpAddr);
        schema.add_field("http.path", Type::String);

        let result = expr_validate_on(&schema, atc, 1024);

        assert!(result.is_err(), "Validation unexcepted success");
        let (err_code, err_message) = result.unwrap_err(); // Unwrap is safe since we've already asserted it
        assert_eq!(
            err_code, ATC_ROUTER_EXPRESSION_VALIDATE_FAILED,
            "Error code mismatch"
        );
        assert_eq!(
            err_message,
            "In/NotIn operators only supports IP in CIDR".to_string(),
            "Error message mismatch"
        );
    }

    #[test]
    fn test_expression_validate_failed_validate() {
        let atc = r##"net.protocol ~ "^https?$" && net.dst.port == 80 && (net.src.ip not in 10.0.0.0/16 || net.src.ip in 10.0.1.0/24) && http.path contains "hello""##;

        let mut schema = Schema::default();
        schema.add_field("net.protocol", Type::String);
        schema.add_field("net.dst.port", Type::Int);
        schema.add_field("net.src.ip", Type::IpAddr);

        let result = expr_validate_on(&schema, atc, 1024);

        assert!(result.is_err(), "Validation unexcepted success");
        let (err_code, err_message) = result.unwrap_err(); // Unwrap is safe since we've already asserted it
        assert_eq!(
            err_code, ATC_ROUTER_EXPRESSION_VALIDATE_FAILED,
            "Error code mismatch"
        );
        assert_eq!(
            err_message,
            "Unknown LHS field".to_string(),
            "Error message mismatch"
        );
    }

    #[test]
    fn test_expression_validate_buf_too_small() {
        let atc = r##"net.protocol ~ "^https?$" && net.dst.port == 80 && (net.src.ip not in 10.0.0.0/16 || net.src.ip in 10.0.1.0/24) && http.path contains "hello""##;

        let mut schema = Schema::default();
        schema.add_field("net.protocol", Type::String);
        schema.add_field("net.dst.port", Type::Int);
        schema.add_field("net.src.ip", Type::IpAddr);
        schema.add_field("http.path", Type::String);

        let result = expr_validate_on(&schema, atc, 46);

        assert!(result.is_err(), "Validation failed");
        let (err_code, _) = result.unwrap_err(); // Unwrap is safe since we've already asserted it
        assert_eq!(
            err_code, ATC_ROUTER_EXPRESSION_VALIDATE_BUF_TOO_SMALL,
            "Error code mismatch"
        );
    }
}
