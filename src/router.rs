use crate::ast::{BinaryOperator, Expression, LogicalExpression, Type};
use crate::context::{Context, Match};
use crate::interpreter::Execute;
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
        let mut mat = Match::new();

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
                    if expr.execute(context, &mut mat) {
                        mat.uuid = key.1;
                        return Some(mat);
                    }
                    mat.reset();
                }
            }
            None => {
                for (MatcherKey(_, id), m) in self.matchers.iter().rev() {
                    if m.execute(context, &mut mat) {
                        mat.uuid = *id;
                        return Some(mat);
                    }

                    mat.reset();
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
    fn failed_and_branch_does_not_leak_state_into_successful_or_branch() {
        let mut schema = Schema::default();
        schema.add_field("http.path", Type::String);
        schema.add_field("http.role", Type::String);
        schema.add_field("http.method", Type::String);

        let mut router = Router::new(&schema);
        router
            .add_matcher(
                0,
                Uuid::default(),
                r##"(http.path ~ r#"^/users/(?<user_id>[^/]+)"# && http.role == "admin") || http.method == "GET""##,
            )
            .expect("should add");

        let mut ctx = Context::new(&schema);
        ctx.add_value("http.path", Value::String("/users/alice".to_owned()));
        ctx.add_value("http.method", Value::String("GET".to_owned()));

        let result = router.try_match(&ctx).expect("method branch should match");
        assert!(result.captures.is_empty());
        assert!(!result.matches.contains_key("http.path"));
        assert_eq!(
            result.matches.get("http.method"),
            Some(&Value::String("GET".to_owned()))
        );
    }

    #[test]
    fn failed_or_alternative_does_not_leak_captures() {
        let mut schema = Schema::default();
        schema.add_field("first", Type::String);
        schema.add_field("guard", Type::String);
        schema.add_field("second", Type::String);

        let mut router = Router::new(&schema);
        router
            .add_matcher(
                0,
                Uuid::default(),
                r##"(first ~ r#"(?<stale>left)"# && guard == "yes") || second ~ r#"(?<winner>right)"#"##,
            )
            .expect("should add");

        let mut ctx = Context::new(&schema);
        ctx.add_value("first", Value::String("left".to_owned()));
        ctx.add_value("second", Value::String("right".to_owned()));

        let result = router.try_match(&ctx).expect("second branch should match");
        assert!(!result.captures.contains_key("stale"));
        assert_eq!(
            result.captures.get("winner").map(String::as_str),
            Some("right")
        );
    }

    #[test]
    fn nested_failed_branch_preserves_enclosing_successful_state() {
        let mut schema = Schema::default();
        schema.add_field("outer", Type::String);
        schema.add_field("inner", Type::String);
        schema.add_field("guard", Type::String);
        schema.add_field("fallback", Type::String);

        let mut router = Router::new(&schema);
        router
            .add_matcher(
                0,
                Uuid::default(),
                r##"outer ~ r#"(?<outer_capture>outside)"# && ((inner ~ r#"(?<inner_capture>inside)"# && guard == "yes") || fallback == "ok")"##,
            )
            .expect("should add");

        let mut ctx = Context::new(&schema);
        ctx.add_value("outer", Value::String("outside".to_owned()));
        ctx.add_value("inner", Value::String("inside".to_owned()));
        ctx.add_value("fallback", Value::String("ok".to_owned()));

        let result = router
            .try_match(&ctx)
            .expect("fallback branch should match");
        assert_eq!(
            result.captures.get("outer_capture").map(String::as_str),
            Some("outside")
        );
        assert!(!result.captures.contains_key("inner_capture"));
        assert!(result.matches.contains_key("outer"));
        assert!(!result.matches.contains_key("inner"));
        assert!(result.matches.contains_key("fallback"));
    }

    #[test]
    fn failed_higher_priority_matcher_does_not_leak_state() {
        let mut schema = Schema::default();
        schema.add_field("candidate", Type::String);
        schema.add_field("fallback", Type::String);

        let higher_priority_id = Uuid::from_u128(1);
        let lower_priority_id = Uuid::from_u128(2);
        let mut router = Router::new(&schema);
        router
            .add_matcher(
                10,
                higher_priority_id,
                r##"candidate ~ r#"^(?<stale>captured)$"#"##,
            )
            .expect("should add higher-priority matcher");
        router
            .add_matcher(
                0,
                lower_priority_id,
                r##"fallback ~ r#"(?<winner>matched)"#"##,
            )
            .expect("should add lower-priority matcher");

        let mut ctx = Context::new(&schema);
        ctx.add_value("candidate", Value::String("captured".to_owned()));
        ctx.add_value("candidate", Value::String("not-captured".to_owned()));
        ctx.add_value("fallback", Value::String("matched".to_owned()));

        let result = router
            .try_match(&ctx)
            .expect("lower-priority matcher should match");
        assert_eq!(result.uuid, lower_priority_id);
        assert!(!result.captures.contains_key("stale"));
        assert_eq!(
            result.captures.get("winner").map(String::as_str),
            Some("matched")
        );
        assert!(!result.matches.contains_key("candidate"));
        assert!(result.matches.contains_key("fallback"));
    }

    #[test]
    fn successful_not_does_not_retain_inner_branch_captures() {
        let mut schema = Schema::default();
        schema.add_field("http.path", Type::String);
        schema.add_field("guard", Type::String);

        let mut router = Router::new(&schema);
        router
            .add_matcher(
                0,
                Uuid::default(),
                r##"!(http.path ~ r#"^/users/(?<user_id>[^/]+)"# && guard == "yes")"##,
            )
            .expect("should add");

        let mut ctx = Context::new(&schema);
        ctx.add_value("http.path", Value::String("/users/alice".to_owned()));

        let result = router.try_match(&ctx).expect("negation should match");
        assert!(result.captures.is_empty());
        assert!(result.matches.is_empty());
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
