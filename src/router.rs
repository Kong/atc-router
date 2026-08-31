use crate::ast::{BinaryOperator, Expression, LogicalExpression, Type};
use crate::context::{Context, Match};
use crate::interpreter::{collect_into, Eval, Matched};
use crate::parser::parse;
use crate::schema::Schema;
use crate::semantics::{FieldCounter, Validate};
use atc_router_prefilter::matchers::{Matcher, MatcherVisitor};
use atc_router_prefilter::{RouterPrefilter, RouterPrefilterIter};
use std::borrow::Borrow;
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MatcherKey(usize, Uuid);

#[derive(Debug)]
struct PrefilteredField {
    field: String,
    // RouterPrefilter returns prefiltered matches in ascending order, but we want to
    // visit them in _descending_ order, so higher priority matches are checked first, so
    // use Reverse to reverse the sort order.
    prefilter: RouterPrefilter<Reverse<MatcherKey>>,
}

#[derive(Debug)]
pub struct Router<S> {
    schema: S,
    matchers: BTreeMap<MatcherKey, Expression>,
    pub fields: HashMap<String, usize>,
    prefiltered_field: Option<PrefilteredField>,
}

impl<S> Router<S>
where
    S: Borrow<Schema>,
{
    /// Creates a new [`Router`] that holds [`Borrow`]<[`Schema`]>.
    ///
    /// This provides flexibility to use different types of schema providers.
    pub fn new(schema: S) -> Self {
        Self {
            schema,
            matchers: BTreeMap::new(),
            fields: HashMap::new(),
            prefiltered_field: None,
        }
    }

    /// Returns a reference to the [`Schema`] used by this router.
    ///
    /// Especially useful when the router owns or wraps the schema,
    /// and you need to pass a reference to other components like [`Context`].
    pub fn schema(&self) -> &Schema {
        self.schema.borrow()
    }

    pub fn add_matcher(&mut self, priority: usize, uuid: Uuid, atc: &str) -> Result<(), String> {
        let expr = parse(atc).map_err(|e| e.to_string())?;

        self.add_matcher_expr(priority, uuid, expr)
    }

    pub fn add_matcher_expr(
        &mut self,
        priority: usize,
        uuid: Uuid,
        expr: Expression,
    ) -> Result<(), String> {
        let key = MatcherKey(priority, uuid);

        if self.matchers.contains_key(&key) {
            return Err("UUID already exists".to_string());
        }

        expr.validate(self.schema())?;
        expr.add_to_counter(&mut self.fields);

        if let Some(filtered_field) = &mut self.prefiltered_field {
            filtered_field.insert(key, &expr);
        }
        assert!(self.matchers.insert(key, expr).is_none());

        Ok(())
    }

    pub fn remove_matcher(&mut self, priority: usize, uuid: Uuid) -> bool {
        let key = MatcherKey(priority, uuid);

        if let Some(filtered_field) = &mut self.prefiltered_field {
            filtered_field.remove(key);
        }

        let Some(ast) = self.matchers.remove(&key) else {
            return false;
        };

        ast.remove_from_counter(&mut self.fields);
        true
    }

    pub fn execute(&self, context: &mut Context) -> bool {
        let Some(m) = self.try_match(context) else {
            return false;
        };

        context.result = Some(m);
        true
    }

    fn prefilter_matches<'a>(
        &'a self,
        context: &'a Context,
    ) -> Option<RouterPrefilterIter<'a, Reverse<MatcherKey>>> {
        let PrefilteredField { field, prefilter } = self.prefiltered_field.as_ref()?;
        if !prefilter.can_prefilter() {
            return None;
        }
        let values = context.value_of(field)?;
        // We only check the first value. We build the pre-filter so that if a matcher uses an
        // `any` transformation, it will be treated as always possibly matching. Otherwise, by
        // default, matchers must match against _all_ values, so our prefilter will return all
        // matchers that could possibly match against the first value. To match against all
        // values, they _must_ be able to match against the first value.
        let value = values.first()?;
        let value = value.as_str()?;
        Some(prefilter.possible_matches(value))
    }

    /// Note that unlike `execute`, this doesn't set `Context.result`
    /// but it also doesn't need a `&mut Context`.
    pub fn try_match(&self, context: &Context) -> Option<Match> {
        // A candidate that fails would only have its matched values and
        // captures discarded, so evaluation just notes what matched and the
        // winner alone pays to turn that into a `Match`. The buffer is reused
        // across candidates, so a walk allocates once. See `interpreter::Eval`.
        let mut noted: Vec<Matched<'_>> = Vec::new();

        let finish = |noted: &[Matched<'_>], uuid: Uuid| {
            let mut mat = Match::new();
            collect_into(noted, &mut mat);
            mat.uuid = uuid;
            mat
        };

        match self.prefilter_matches(context) {
            Some(possible_matches) => {
                for key in possible_matches {
                    let key = &key.0;
                    let Some(expr) = self.matchers.get(key) else {
                        if cfg!(debug_assertions) {
                            unreachable!("prefilter cannot return a matcher key not in `matchers`");
                        }
                        continue;
                    };
                    noted.clear();
                    if expr.eval(context, &mut noted) {
                        return Some(finish(&noted, key.1));
                    }
                }
            }
            None => {
                for (MatcherKey(_, id), m) in self.matchers.iter().rev() {
                    noted.clear();
                    if m.eval(context, &mut noted) {
                        return Some(finish(&noted, *id));
                    }
                }
            }
        }

        None
    }

    /// Enable prefiltering on the specified field.
    ///
    /// This will compile a prefilter for all currently existing matchers, and all future added
    /// matchers will be added to the prefilter. This can be an expensive call if there are a lot
    /// of matchers.
    ///
    /// If possible, it is slightly more efficient to call this _after_ all matchers have been
    /// added, rather than enabling at construction time and building the prefilter as each matcher
    /// is added.
    pub fn enable_prefilter(&mut self, field: &str) -> Result<(), String> {
        if let Some(prefiltered_field) = &self.prefiltered_field {
            if prefiltered_field.field == field {
                // Already prefiltered by this field
                return Ok(());
            }
        }
        match self.schema.borrow().type_of(field) {
            Some(Type::String) => {}
            Some(actual) => {
                return Err(format!(
                    "Field {field} is of type {actual:?}, must be a string"
                ))
            }
            None => return Err(format!("Field {field} is not in schema")),
        }
        let mut prefilter = RouterPrefilter::new();
        for (key, expr) in &self.matchers {
            prefilter.insert(Reverse(*key), ExprMatcher { expr, field });
        }
        self.prefiltered_field = Some(PrefilteredField {
            field: field.to_string(),
            prefilter,
        });
        Ok(())
    }

    /// Disable prefiltering.
    pub fn disable_prefilter(&mut self) {
        self.prefiltered_field = None;
    }
}

impl PrefilteredField {
    fn insert(&mut self, key: MatcherKey, expr: &Expression) {
        self.prefilter.insert(
            Reverse(key),
            ExprMatcher {
                expr,
                field: &self.field,
            },
        );
    }

    fn remove(&mut self, key: MatcherKey) {
        self.prefilter.remove(&Reverse(key));
    }
}

struct ExprMatcher<'a> {
    expr: &'a Expression,
    field: &'a str,
}

impl Matcher for ExprMatcher<'_> {
    fn visit(&self, visitor: &mut MatcherVisitor) {
        match self.expr {
            Expression::Logical(logical) => {
                visitor.visit_nested_start();
                match logical.as_ref() {
                    LogicalExpression::And(lhs, rhs) => {
                        let left_matcher = Self {
                            expr: lhs,
                            field: self.field,
                        };
                        let right_matcher = Self {
                            expr: rhs,
                            field: self.field,
                        };
                        left_matcher.visit(visitor);
                        right_matcher.visit(visitor);
                    }
                    LogicalExpression::Or(lhs, rhs) => {
                        let left_matcher = Self {
                            expr: lhs,
                            field: self.field,
                        };
                        let right_matcher = Self {
                            expr: rhs,
                            field: self.field,
                        };
                        left_matcher.visit(visitor);
                        visitor.visit_or_in();
                        right_matcher.visit(visitor);
                    }
                    LogicalExpression::Not(_inner) => {
                        // can't visit
                    }
                }
                visitor.visit_nested_finish();
            }
            Expression::Predicate(pred) => {
                if pred.lhs.var_name == self.field && pred.lhs.transformations.is_empty() {
                    match pred.op {
                        BinaryOperator::Equals => {
                            let rhs = pred
                                .rhs
                                .as_str()
                                .expect("can only use a prefilter on strings");
                            visitor.visit_match_equals(rhs);
                        }
                        BinaryOperator::Prefix => {
                            let rhs = pred
                                .rhs
                                .as_str()
                                .expect("can only use a prefilter on strings");
                            visitor.visit_match_starts_with(rhs);
                        }
                        BinaryOperator::Regex => {
                            let rhs = pred
                                .rhs
                                .as_regex()
                                .expect("can only use a prefilter on strings");
                            visitor.visit_match_regex(rhs.as_str());
                        }
                        BinaryOperator::NotEquals
                        | BinaryOperator::Postfix
                        | BinaryOperator::Greater
                        | BinaryOperator::GreaterOrEqual
                        | BinaryOperator::Less
                        | BinaryOperator::LessOrEqual
                        | BinaryOperator::In
                        | BinaryOperator::NotIn
                        | BinaryOperator::Contains => {}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::{
        ast::{MatchedValue, Type, Value},
        context::Context,
        schema::Schema,
    };

    use super::Router;

    use std::sync::Arc;

    fn make_uuid(a: usize) -> Uuid {
        format!("8cb2a7d0-c775-4ed9-989f-{:012}", a)
            .parse()
            .unwrap()
    }

    #[test]
    fn execute_succeeds() {
        let mut schema = Schema::default();
        schema.add_field("http.path", Type::String);

        let mut router = Router::new(&schema);
        router
            .add_matcher(0, Uuid::default(), "http.path == \"/dev\"")
            .expect("should add");

        let mut ctx = Context::new(&schema);
        ctx.add_value("http.path", "/dev".to_owned().into());
        assert!(router.execute(&mut ctx));
    }

    #[test]
    fn execute_fails() {
        let mut schema = Schema::default();
        schema.add_field("http.path", Type::String);

        let mut router = Router::new(&schema);
        router
            .add_matcher(0, Uuid::default(), "http.path == \"/dev\"")
            .expect("should add");

        let mut ctx = Context::new(&schema);
        ctx.add_value("http.path", "/not-dev".to_owned().into());
        assert!(!router.execute(&mut ctx));
    }

    #[test]
    fn try_match_succeeds() {
        let mut schema = Schema::default();
        schema.add_field("http.path", Type::String);

        let mut router = Router::new(&schema);
        router
            .add_matcher(0, Uuid::default(), "http.path == \"/dev\"")
            .expect("should add");

        let mut ctx = Context::new(&schema);
        ctx.add_value("http.path", "/dev".to_owned().into());
        router.try_match(&ctx).expect("matches");
    }

    #[test]
    fn try_match_fails() {
        let mut schema = Schema::default();
        schema.add_field("http.path", Type::String);

        let mut router = Router::new(&schema);
        router
            .add_matcher(0, Uuid::default(), "http.path == \"/dev\"")
            .expect("should add");

        let mut ctx = Context::new(&schema);
        ctx.add_value("http.path", "/not-dev".to_owned().into());
        router.try_match(&ctx).ok_or(()).expect_err("should fail");
    }

    #[test]
    fn test_shared_schema_instantiation() {
        let mut schema = Schema::default();
        schema.add_field("http.path", Type::String);

        let mut router = Router::new(&schema);
        router
            .add_matcher(0, Uuid::default(), "http.path == \"/dev\"")
            .expect("should add");
        let mut ctx = Context::new(router.schema());
        ctx.add_value("http.path", "/dev".to_owned().into());
        router.try_match(&ctx).expect("matches");
    }

    #[test]
    fn test_owned_schema_instantiation() {
        let mut schema = Schema::default();
        schema.add_field("http.path", Type::String);

        let mut router = Router::new(schema);
        router
            .add_matcher(0, Uuid::default(), "http.path == \"/dev\"")
            .expect("should add");
        let mut ctx = Context::new(router.schema());
        ctx.add_value("http.path", "/dev".to_owned().into());
        router.try_match(&ctx).expect("matches");
    }

    #[test]
    fn test_arc_schema_instantiation() {
        let mut schema = Schema::default();
        schema.add_field("http.path", Type::String);

        let mut router = Router::new(Arc::new(schema));
        router
            .add_matcher(0, Uuid::default(), "http.path == \"/dev\"")
            .expect("should add");
        let mut ctx = Context::new(router.schema());
        ctx.add_value("http.path", "/dev".to_owned().into());
        router.try_match(&ctx).expect("matches");
    }

    #[test]
    fn test_box_schema_instantiation() {
        let mut schema = Schema::default();
        schema.add_field("http.path", Type::String);

        let mut router = Router::new(Box::new(schema));
        router
            .add_matcher(0, Uuid::default(), "http.path == \"/dev\"")
            .expect("should add");
        let mut ctx = Context::new(router.schema());
        ctx.add_value("http.path", "/dev".to_owned().into());
        router.try_match(&ctx).expect("matches");
    }

    #[test]
    fn test_matched_expr_prefix() {
        let mut schema = Schema::default();
        schema.add_field("http.path", Type::String);

        let mut router = Router::new(&schema);
        router
            .add_matcher(
                0,
                Uuid::default(),
                "http.path ^= \"/abc\" || http.path ^= \"/foo\"",
            )
            .expect("should add");

        let mut ctx = Context::new(&schema);
        ctx.add_value("http.path", "/foo/bar".to_owned().into());
        assert!(router.execute(&mut ctx));

        let res = ctx.result.as_ref().unwrap();
        assert_eq!(
            res.matches.get("http.path").map(MatchedValue::expression),
            Some(&Value::String("/foo".to_string())),
        );

        ctx.reset();
        ctx.add_value("http.path", "/abc/xyz".to_owned().into());
        assert!(router.execute(&mut ctx));

        let res = ctx.result.as_ref().unwrap();
        assert_eq!(
            res.matches.get("http.path").map(MatchedValue::expression),
            Some(&Value::String("/abc".to_string())),
        );
    }

    #[test]
    fn test_matched_expr_regex() {
        let mut schema = Schema::default();
        schema.add_field("http.path", Type::String);

        let mut router = Router::new(&schema);
        router
            .add_matcher(
                0,
                Uuid::default(),
                "http.path ~ r#\"^/\\d+/test$\"# || http.path ~ r#\"^/\\d+/bar$\"#",
            )
            .expect("should add");

        let mut ctx = Context::new(&schema);
        ctx.add_value("http.path", "/123/test".to_owned().into());
        assert!(router.execute(&mut ctx));

        let res = ctx.result.as_ref().unwrap();
        // the `expression` side stores the raw regex pattern.
        assert_eq!(
            res.matches.get("http.path").map(MatchedValue::expression),
            Some(&Value::String("^/\\d+/test$".to_string()))
        );
        // the `value` side stores the substring of the request value that matched.
        assert_eq!(
            res.matches.get("http.path").map(MatchedValue::value),
            Some(&Value::String("/123/test".to_string()))
        );

        ctx.reset();
        ctx.add_value("http.path", "/123/bar".to_owned().into());
        assert!(router.execute(&mut ctx));

        let res = ctx.result.as_ref().unwrap();
        // the `expression` side stores the raw regex pattern.
        assert_eq!(
            res.matches.get("http.path").map(MatchedValue::expression),
            Some(&Value::String("^/\\d+/bar$".to_string()))
        );
        // the `value` side stores the substring of the request value that matched.
        assert_eq!(
            res.matches.get("http.path").map(MatchedValue::value),
            Some(&Value::String("/123/bar".to_string()))
        );
    }

    /// A candidate that matches part of its expression and then fails must not
    /// leave anything behind for the route that eventually wins.
    #[test]
    fn test_failed_candidate_leaves_no_residue() {
        let mut schema = Schema::default();
        schema.add_field("http.host", Type::String);
        schema.add_field("http.path", Type::String);

        let mut router = Router::new(&schema);
        // Higher priority: the host matches, then the path fails.
        router
            .add_matcher(
                10,
                make_uuid(1),
                r#"http.host == "a.test" && http.path ^= "/nope""#,
            )
            .expect("should add");
        // Lower priority: this one wins.
        router
            .add_matcher(
                1,
                make_uuid(2),
                r#"http.host == "a.test" && http.path ^= "/yes""#,
            )
            .expect("should add");

        let mut ctx = Context::new(&schema);
        ctx.add_value("http.host", "a.test".to_owned().into());
        ctx.add_value("http.path", "/yes/please".to_owned().into());

        assert!(router.execute(&mut ctx));
        let res = ctx.result.as_ref().unwrap();

        assert_eq!(res.uuid, make_uuid(2));
        // The winner's own prefix, not the loser's.
        assert_eq!(
            res.matches.get("http.path").map(MatchedValue::expression),
            Some(&Value::String("/yes".to_string())),
        );
        assert_eq!(
            res.matches.get("http.host").map(MatchedValue::expression),
            Some(&Value::String("a.test".to_string())),
        );
        assert_eq!(res.matches.len(), 2);
        assert!(res.captures.is_empty());
    }

    /// Regex captures belong to the winning route only, and survive a losing
    /// candidate that also ran a matching regex.
    #[test]
    fn test_captures_come_from_the_winner_only() {
        let mut schema = Schema::default();
        schema.add_field("http.host", Type::String);
        schema.add_field("http.path", Type::String);

        let mut router = Router::new(&schema);
        // Higher priority: the regex matches and captures, then the host fails.
        router
            .add_matcher(
                10,
                make_uuid(1),
                r##"http.path ~ r#"^/(loser)/(\d+)$"# && http.host == "other.test""##,
            )
            .expect("should add");
        router
            .add_matcher(
                1,
                make_uuid(2),
                r##"http.path ~ r#"^/(?P<who>winner)/(\d+)$"#"##,
            )
            .expect("should add");

        let mut ctx = Context::new(&schema);
        ctx.add_value("http.host", "a.test".to_owned().into());
        ctx.add_value("http.path", "/winner/42".to_owned().into());

        assert!(router.execute(&mut ctx));
        let res = ctx.result.as_ref().unwrap();

        assert_eq!(res.uuid, make_uuid(2));
        assert_eq!(res.captures.get("1").map(String::as_str), Some("winner"));
        assert_eq!(res.captures.get("2").map(String::as_str), Some("42"));
        assert_eq!(res.captures.get("who").map(String::as_str), Some("winner"));
        // Nothing from the losing candidate.
        assert!(!res.captures.values().any(|v| v == "loser"));
    }

    /// The second, collecting run must take the same `||` branch the deciding
    /// run took, so the recorded value is the branch that actually matched.
    #[test]
    fn test_or_branch_agrees_between_runs() {
        let mut schema = Schema::default();
        schema.add_field("http.path", Type::String);

        let mut router = Router::new(&schema);
        router
            .add_matcher(
                0,
                make_uuid(1),
                r#"http.path ^= "/alpha" || http.path ^= "/beta""#,
            )
            .expect("should add");

        for (path, expected) in [("/alpha/x", "/alpha"), ("/beta/x", "/beta")] {
            let mut ctx = Context::new(&schema);
            ctx.add_value("http.path", path.to_owned().into());

            assert!(router.execute(&mut ctx));
            let res = ctx.result.as_ref().unwrap();
            assert_eq!(
                res.matches.get("http.path").map(MatchedValue::expression),
                Some(&Value::String(expected.to_string())),
                "path {path}"
            );
        }
    }

    #[test]
    fn test_matched_expr_absent_on_no_match() {
        let mut schema = Schema::default();
        schema.add_field("http.path", Type::String);

        let mut router = Router::new(&schema);
        router
            .add_matcher(
                0,
                Uuid::default(),
                "http.path ^= \"/foo\" || http.path ^= \"/bar\"",
            )
            .expect("should add");

        let mut ctx = Context::new(&schema);
        ctx.add_value("http.path", "/nope".to_owned().into());
        assert!(!router.execute(&mut ctx));
        assert!(ctx.result.is_none());
    }
}
