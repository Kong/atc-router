use crate::ast::Expression;
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::parser::parse;
use crate::schema::Schema;
use crate::semantics::{FieldCounter, Validate};
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MatcherKey(usize, Uuid);

#[derive(Debug)]
pub struct Router<S> {
    schema: S,
    matchers: BTreeMap<MatcherKey, Expression>,
    pub fields: HashMap<String, usize>,
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

        assert!(self.matchers.insert(key, expr).is_none());

        Ok(())
    }

    pub fn remove_matcher(&mut self, priority: usize, uuid: Uuid) -> bool {
        let key = MatcherKey(priority, uuid);

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

    /// Note that unlike `execute`, this doesn't set `Context.result`
    /// but it also doesn't need a `&mut Context`.
    pub fn try_match(&self, context: &Context) -> Option<Match> {
        let mut mat = Match::new();

        for (MatcherKey(_, id), m) in self.matchers.iter().rev() {
            if m.execute(context, &mut mat) {
                mat.uuid = *id;
                return Some(mat);
            }

            mat.reset();
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::{
        ast::{Type, Value},
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
}
