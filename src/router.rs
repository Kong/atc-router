use crate::ast::Expression;
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::parser::parse;
use crate::schema::Schema;
use crate::semantics::{FieldCounter, Validate};
use std::collections::{BTreeMap, HashMap};
use std::ops::Deref;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MatcherKey(usize, Uuid);

#[derive(Debug)]
pub struct Router<'a> {
    schema: SchemaOwnedOrRef<'a>,
    matchers: BTreeMap<MatcherKey, Expression>,
    pub fields: HashMap<String, usize>,
}

impl<'a> Router<'a> {
    /// Creates a new [`Router`] that holds a shared reference to a [`Schema`].
    ///
    /// This is useful when the schema is managed outside the router and/or shared
    /// across multiple components.
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema: SchemaOwnedOrRef::Ref(schema),
            matchers: BTreeMap::new(),
            fields: HashMap::new(),
        }
    }

    /// Creates a new [`Router`] that owns its [`Schema`].
    ///
    /// This allows the router to be self contained,
    /// making it easier to use as a standalone component.
    pub fn new_owning(schema: Schema) -> Self {
        Self {
            schema: SchemaOwnedOrRef::Owned(schema),
            matchers: BTreeMap::new(),
            fields: HashMap::new(),
        }
    }

    /// Returns a reference to the [`Schema`] used by this router.
    ///
    /// Especially useful if the router owns the schema internally ([`new_owning`]),
    /// but you still need to pass a reference to other components like [`Context`].
    ///
    /// [`new_owning`]: Router::new_owning
    pub fn schema(&self) -> &Schema {
        &self.schema
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

        expr.validate(&self.schema)?;
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

/// A smart pointer over a [`Schema`], which may be either borrowed or owned.
///
/// Used by [`Router`] to support both externally managed and self-contained schemas.
/// Owning the schema is especially useful when the router is used outside of the FFI context,
/// making it fully independent.
///
/// Implements [`Deref`] for ergonomic access to the underlying [`Schema`].
#[derive(Debug)]
enum SchemaOwnedOrRef<'a> {
    Ref(&'a Schema),
    Owned(Schema),
}

impl Deref for SchemaOwnedOrRef<'_> {
    type Target = Schema;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Ref(s) => s,
            Self::Owned(s) => s,
        }
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::{ast::Type, context::Context, schema::Schema};

    use super::Router;

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
    fn test_basic_owned_schema() {
        let mut schema = Schema::default();
        schema.add_field("http.path", Type::String);

        let mut router: Router<'static> = Router::new_owning(schema);
        router
            .add_matcher(0, Uuid::default(), "http.path == \"/dev\"")
            .expect("should add");
        let mut ctx = Context::new(router.schema());
        ctx.add_value("http.path", "/dev".to_owned().into());
        router.try_match(&ctx).expect("matches");
    }
}
