use crate::ast::{Expression, LogicalExpression, Value};
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::parser::parse;
use crate::schema::Schema;
use crate::semantics::{FieldCounter, Validate};
use regex::Regex;
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MatcherKey(usize, Uuid);

#[derive(Debug)]
pub struct Router<S> {
    schema: S,
    matchers: BTreeMap<MatcherKey, Expression>,
    pub fields: HashMap<String, usize>,
    regex_cache: HashMap<String, Rc<Regex>>,
}

fn release_cache<S>(expr: &Expression, router: &mut Router<S>) {
    match expr {
        Expression::Logical(l) => match l.as_ref() {
            LogicalExpression::And(l, r) | LogicalExpression::Or(l, r) => {
                release_cache(l, router);
                release_cache(r, router);
            }
            LogicalExpression::Not(r) => release_cache(r, router),
        },
        Expression::Predicate(p) => {
            if let Value::Regex(rc) = &p.rhs {
                if Rc::strong_count(rc) == 2 {
                    // about to be dropped and the only Rc left is in the map
                    router.regex_cache.remove(rc.as_str());
                }
            }
        }
    };
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
            regex_cache: HashMap::new(),
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
        let expr = parse(atc, &mut self.regex_cache).map_err(|e| e.to_string())?;

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

        release_cache(&ast, self);
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

    use crate::{ast::Type, context::Context, schema::Schema};

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
