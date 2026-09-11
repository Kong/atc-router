use crate::ast::{
    BinaryOperator, Expression, LogicalExpression, MatchedValue, Predicate, RegexMatchedValue,
    Value,
};
use crate::context::{Context, Match};
use std::borrow::Cow;

pub trait Execute {
    fn execute(&self, ctx: &Context, m: &mut Match) -> bool;
}

/// A predicate that matched, noted during evaluation instead of being recorded
/// into a [`Match`] straight away.
///
/// Recording a matched value costs a `String` clone for the field name and a
/// `Value` clone for the expression side; a matching regex additionally runs
/// `captures()`. A router visits many candidates and keeps at most one, so
/// paying that on every candidate throws almost all of it away. Noting a
/// reference costs a `Vec` push instead, and only the expression that wins is
/// turned into a `Match` (see [`collect_into`]).
pub(crate) enum Matched<'a> {
    Plain(&'a Predicate),
    /// The subject the regex matched, kept so `captures()` runs once, at commit
    /// time. It borrows the context value except under a `lower` transformation,
    /// which has no borrowable original.
    Regex {
        pred: &'a Predicate,
        subject: Cow<'a, str>,
    },
}

/// Turns the predicates noted during evaluation into a [`Match`].
///
/// Order matters: later entries overwrite earlier ones for the same field,
/// which is what recording inline during evaluation also did.
pub(crate) fn collect_into(matched: &[Matched<'_>], m: &mut Match) {
    for entry in matched {
        match entry {
            Matched::Plain(pred) => {
                m.matches.insert(
                    pred.lhs.var_name.clone(),
                    MatchedValue::Plain(pred.rhs.clone()),
                );
            }
            Matched::Regex { pred, subject } => {
                // SAFETY: the predicate matched during evaluation, so the regex
                // and the subject both still apply.
                let rhs = pred.rhs.as_regex().unwrap();
                let reg_cap = rhs.captures(subject).unwrap();

                m.matches.insert(
                    pred.lhs.var_name.clone(),
                    MatchedValue::Regex(Box::new(RegexMatchedValue {
                        captured: Value::String(reg_cap.get(0).unwrap().as_str().to_string()),
                        expr: Value::String(rhs.as_str().to_string()),
                    })),
                );

                for (i, c) in reg_cap.iter().enumerate() {
                    if let Some(c) = c {
                        m.captures.insert(i.to_string(), c.as_str().to_string());
                    }
                }

                // named captures
                for n in rhs.capture_names().flatten() {
                    if let Some(value) = reg_cap.name(n) {
                        m.captures.insert(n.to_string(), value.as_str().to_string());
                    }
                }
            }
        }
    }
}

/// Noting a match is off the comparison path: a walk performs one comparison
/// per candidate and notes something only when a predicate actually matches, so
/// keeping this out of line leaves `Predicate::eval` small enough to inline.
#[cold]
#[inline(never)]
fn note_plain<'a>(out: &mut Vec<Matched<'a>>, pred: &'a Predicate) {
    out.push(Matched::Plain(pred));
}

/// Evaluation that notes what matched rather than recording it.
pub(crate) trait Eval {
    fn eval<'a>(&'a self, ctx: &'a Context, out: &mut Vec<Matched<'a>>) -> bool;
}

impl Execute for Expression {
    fn execute(&self, ctx: &Context, m: &mut Match) -> bool {
        let mut out = Vec::new();
        let matched = self.eval(ctx, &mut out);
        // Recorded whatever the outcome, because recording inline did the same:
        // a caller that keeps a `Match` from a false expression still sees the
        // predicates that matched along the way.
        collect_into(&out, m);
        matched
    }
}

impl Execute for Predicate {
    fn execute(&self, ctx: &Context, m: &mut Match) -> bool {
        let mut out = Vec::new();
        let matched = self.eval(ctx, &mut out);
        collect_into(&out, m);
        matched
    }
}

impl Eval for Expression {
    fn eval<'a>(&'a self, ctx: &'a Context, out: &mut Vec<Matched<'a>>) -> bool {
        match self {
            Expression::Logical(l) => match l.as_ref() {
                LogicalExpression::And(l, r) => l.eval(ctx, out) && r.eval(ctx, out),
                LogicalExpression::Or(l, r) => l.eval(ctx, out) || r.eval(ctx, out),
                LogicalExpression::Not(r) => !r.eval(ctx, out),
            },
            Expression::Predicate(p) => p.eval(ctx, out),
        }
    }
}

impl Eval for Predicate {
    fn eval<'a>(&'a self, ctx: &'a Context, out: &mut Vec<Matched<'a>>) -> bool {
        let lhs_values = match ctx.value_of(&self.lhs.var_name) {
            None => return false,
            Some(v) => v,
        };

        let (lower, any) = self.lhs.get_transformations();

        // can only be "all" or "any" mode.
        // - all: all values must match (default)
        // - any: ok if any any matched
        for original in lhs_values.iter() {
            let mut lhs_value = original;
            let lhs_value_transformed;

            if lower {
                // SAFETY: this only panic if and only if
                // the semantic checking didn't catch the mismatched types,
                // which is a bug.
                let s = lhs_value.as_str().unwrap();

                lhs_value_transformed = Value::String(s.to_lowercase());
                lhs_value = &lhs_value_transformed;
            }

            let mut matched = false;
            match self.op {
                BinaryOperator::Equals => {
                    if lhs_value == &self.rhs {
                        note_plain(out, self);

                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::NotEquals => {
                    if lhs_value != &self.rhs {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::Regex => {
                    // SAFETY: this only panic if and only if
                    // the semantic checking didn't catch the mismatched types,
                    // which is a bug.
                    let lhs = lhs_value.as_str().unwrap();
                    let rhs = self.rhs.as_regex().unwrap();

                    if rhs.is_match(lhs) {
                        // `captures()` is a second, much more expensive run of
                        // the regex. It is deferred to `collect_into`, so only
                        // the winning expression pays for it.
                        out.push(Matched::Regex {
                            pred: self,
                            subject: if lower {
                                Cow::Owned(lhs.to_string())
                            } else {
                                // SAFETY: without a transformation `lhs_value`
                                // is `original`, whose data lives in `ctx`.
                                Cow::Borrowed(original.as_str().unwrap())
                            },
                        });

                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::Prefix => {
                    // SAFETY: this only panic if and only if
                    // the semantic checking didn't catch the mismatched types,
                    // which is a bug.
                    let lhs = lhs_value.as_str().unwrap();
                    let rhs = self.rhs.as_str().unwrap();

                    if lhs.starts_with(rhs) {
                        note_plain(out, self);
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::Postfix => {
                    // SAFETY: this only panic if and only if
                    // the semantic checking didn't catch the mismatched types,
                    // which is a bug.
                    let lhs = lhs_value.as_str().unwrap();
                    let rhs = self.rhs.as_str().unwrap();

                    if lhs.ends_with(rhs) {
                        note_plain(out, self);
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::Greater => {
                    // SAFETY: this only panic if and only if
                    // the semantic checking didn't catch the mismatched types,
                    // which is a bug.
                    let lhs = lhs_value.as_int().unwrap();
                    let rhs = self.rhs.as_int().unwrap();

                    if lhs > rhs {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::GreaterOrEqual => {
                    // SAFETY: this only panic if and only if
                    // the semantic checking didn't catch the mismatched types,
                    // which is a bug.
                    let lhs = lhs_value.as_int().unwrap();
                    let rhs = self.rhs.as_int().unwrap();

                    if lhs >= rhs {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::Less => {
                    // SAFETY: this only panic if and only if
                    // the semantic checking didn't catch the mismatched types,
                    // which is a bug.
                    let lhs = lhs_value.as_int().unwrap();
                    let rhs = self.rhs.as_int().unwrap();

                    if lhs < rhs {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::LessOrEqual => {
                    // SAFETY: this only panic if and only if
                    // the semantic checking didn't catch the mismatched types,
                    // which is a bug.
                    let lhs = lhs_value.as_int().unwrap();
                    let rhs = self.rhs.as_int().unwrap();

                    if lhs <= rhs {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
                BinaryOperator::In => {
                    // SAFETY: this only panic if and only if
                    // the semantic checking didn't catch the mismatched types,
                    // which is a bug.
                    let lhs = lhs_value.as_ipaddr().unwrap();
                    let rhs = self.rhs.as_ipcidr().unwrap();

                    if rhs.contains(lhs) {
                        matched = true;
                        if any {
                            return true;
                        }
                    }
                }
                BinaryOperator::NotIn => {
                    // SAFETY: this only panic if and only if
                    // the semantic checking didn't catch the mismatched types,
                    // which is a bug.
                    let lhs = lhs_value.as_ipaddr().unwrap();
                    let rhs = self.rhs.as_ipcidr().unwrap();

                    if !rhs.contains(lhs) {
                        matched = true;
                        if any {
                            return true;
                        }
                    }
                }
                BinaryOperator::Contains => {
                    // SAFETY: this only panic if and only if
                    // the semantic checking didn't catch the mismatched types,
                    // which is a bug.
                    let lhs = lhs_value.as_str().unwrap();
                    let rhs = self.rhs.as_str().unwrap();

                    if lhs.contains(rhs) {
                        if any {
                            return true;
                        }

                        matched = true;
                    }
                }
            } // match

            if !any && !matched {
                // all and nothing matched
                return false;
            }
        } // for iter

        // if we reached here, it means that `any` did not find a match,
        // or we passed all matches for `all`. So we simply need to return
        // !any && lhs_values.len() > 0 to cover both cases
        !any && !lhs_values.is_empty()
    }
}

#[test]
fn test_predicate() {
    use crate::ast;
    use crate::schema;

    let mut mat = Match::new();
    let mut schema = schema::Schema::default();
    schema.add_field("my_key", ast::Type::String);
    let mut ctx = Context::new(&schema);

    // check when value list is empty
    // check if all values match starts_with foo -- should be false
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert!(!p.execute(&mut ctx, &mut mat));

    // check if any value matches starts_with foo -- should be false
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert!(!p.execute(&mut ctx, &mut mat));

    // test any mode
    let lhs_values = vec![
        Value::String("foofoo".to_string()),
        Value::String("foobar".to_string()),
        Value::String("foocar".to_string()),
        Value::String("fooban".to_string()),
    ];

    for v in lhs_values {
        ctx.add_value("my_key", v);
    }

    // check if all values match starts_with foo -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert!(p.execute(&mut ctx, &mut mat));

    // check if all values match ends_with foo -- should be false
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert!(!p.execute(&mut ctx, &mut mat));

    // check if any value matches ends_with foo -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert!(p.execute(&mut ctx, &mut mat));

    // check if any value matches starts_with foo -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("foo".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert!(p.execute(&mut ctx, &mut mat));

    // check if any value matches ends_with nar -- should be false
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("nar".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert!(!p.execute(&mut ctx, &mut mat));

    // check if any value matches ends_with empty string -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("".to_string()),
        op: BinaryOperator::Postfix,
    };

    assert!(p.execute(&mut ctx, &mut mat));

    // check if any value matches starts_with empty string -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("".to_string()),
        op: BinaryOperator::Prefix,
    };

    assert!(p.execute(&mut ctx, &mut mat));

    // check if any value matches contains `ob` -- should be true
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("ob".to_string()),
        op: BinaryOperator::Contains,
    };

    assert!(p.execute(&mut ctx, &mut mat));

    // check if any value matches contains `ok` -- should be false
    let p = Predicate {
        lhs: ast::Lhs {
            var_name: "my_key".to_string(),
            transformations: vec![ast::LhsTransformations::Any],
        },
        rhs: Value::String("ok".to_string()),
        op: BinaryOperator::Contains,
    };

    assert!(!p.execute(&mut ctx, &mut mat));
}
