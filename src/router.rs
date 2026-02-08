use crate::ast::{BinaryOperator, Expression, LogicalExpression, Type};
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::parser::parse;
use crate::schema::Schema;
use crate::semantics::{FieldCounter, Validate};
use router_prefilter::matchers::{Matcher, MatcherVisitor};
use router_prefilter::{RouterPrefilter, RouterPrefilterIter};
use std::borrow::Borrow;
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MatcherKey(usize, Uuid);

#[derive(Debug)]
struct PrefilteredField {
    field: String,
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
        match &self.prefiltered_field {
            Some(PrefilteredField { field, prefilter }) if prefilter.can_prefilter() => {
                let values = context.value_of(field)?;
                let value = values.first()?;
                let value = value.as_str()?;
                Some(prefilter.possible_matches(value))
            }
            _ => None,
        }
    }

    /// Note that unlike `execute`, this doesn't set `Context.result`
    /// but it also doesn't need a `&mut Context`.
    pub fn try_match(&self, context: &Context) -> Option<Match> {
        let mut mat = Match::new();

        match self.prefilter_matches(context) {
            Some(possible_matches) => {
                for key in possible_matches {
                    let key = &key.0;
                    let expr = &self.matchers[key];
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
    pub fn enable_prefilter(&mut self, field: &str) {
        match self.schema.borrow().type_of(field) {
            Some(Type::String) => {}
            Some(actual) => panic!("Field {field} is of type {actual:?}, must be a string"),
            None => panic!("Field {field} is not in schema"),
        }
        let mut prefilter = RouterPrefilter::new();
        for (key, expr) in &self.matchers {
            prefilter.insert(Reverse(key.clone()), ExprMatcher { expr, field });
        }
        self.prefiltered_field = Some(PrefilteredField {
            field: field.to_string(),
            prefilter,
        });
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
