use crate::ast::Expression;
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::parser::parse;
use crate::schema::Schema;
use crate::semantics::{FieldCounter, Validate};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MatcherKey(usize, Uuid);

#[derive(Debug)]
enum SchemaOwnedOrRef<'a> {
    Ref(&'a Schema),
    Owned(Schema),
}

impl AsRef<Schema> for SchemaOwnedOrRef<'_> {
    fn as_ref(&self) -> &Schema {
        match self {
            SchemaOwnedOrRef::Ref(s) => s,
            SchemaOwnedOrRef::Owned(s) => s,
        }
    }
}

#[derive(Debug)]
pub struct Router<'a> {
    schema: SchemaOwnedOrRef<'a>,
    matchers: BTreeMap<MatcherKey, Expression>,
    pub fields: HashMap<String, usize>,
}

impl<'a> Router<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema: SchemaOwnedOrRef::Ref(schema),
            matchers: BTreeMap::new(),
            fields: HashMap::new(),
        }
    }

    pub fn new_owning(schema: Schema) -> Self {
        Self {
            schema: SchemaOwnedOrRef::Owned(schema),
            matchers: BTreeMap::new(),
            fields: HashMap::new(),
        }
    }

    pub fn schema(&self) -> &Schema {
        self.schema.as_ref()
    }

    pub fn add_matcher(&mut self, priority: usize, uuid: Uuid, atc: &str) -> Result<(), String> {
        let key = MatcherKey(priority, uuid);

        if self.matchers.contains_key(&key) {
            return Err("UUID already exists".to_string());
        }

        let ast = parse(atc).map_err(|e| e.to_string())?;

        ast.validate(self.schema.as_ref())?;
        ast.add_to_counter(&mut self.fields);

        assert!(self.matchers.insert(key, ast).is_none());

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
        for (MatcherKey(_, id), m) in self.matchers.iter().rev() {
            let mut mat = Match::new();
            if m.execute(context, &mut mat) {
                mat.uuid = *id;
                return Some(mat);
            }
        }

        None
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
