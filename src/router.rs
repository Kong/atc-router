use crate::ast::{Expression, LogicalExpression, Value};
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::parser::parse;
use crate::schema::Schema;
use crate::semantics::{FieldCounter, Validate};
use regex::Regex;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use uuid::Uuid;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct MatcherKey(usize, Uuid);

pub struct Router<'a> {
    schema: &'a Schema,
    matchers: BTreeMap<MatcherKey, Expression>,
    pub fields: HashMap<String, usize>,
    regex_cache: HashMap<String, Rc<Regex>>,
}

fn release_cache(expr: &Expression, router: &mut Router) {
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

impl<'a> Router<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            matchers: BTreeMap::new(),
            fields: HashMap::new(),
            regex_cache: HashMap::new(),
        }
    }

    pub fn add_matcher(&mut self, priority: usize, uuid: Uuid, atc: &str) -> Result<(), String> {
        let key = MatcherKey(priority, uuid);

        if self.matchers.contains_key(&key) {
            return Err("UUID already exists".to_string());
        }

        let ast = parse(atc, &mut self.regex_cache).map_err(|e| e.to_string())?;

        ast.validate(self.schema)?;
        ast.add_to_counter(&mut self.fields);

        assert!(self.matchers.insert(key, ast).is_none());

        Ok(())
    }

    pub fn remove_matcher(&mut self, priority: usize, uuid: Uuid) -> bool {
        let key = MatcherKey(priority, uuid);

        if let Some(ast) = self.matchers.remove(&key) {
            release_cache(&ast, self);
            ast.remove_from_counter(&mut self.fields);
            return true;
        }

        false
    }

    pub fn execute(&self, context: &mut Context) -> bool {
        for (MatcherKey(_, id), m) in self.matchers.iter().rev() {
            let mut mat = Match::new();
            if m.execute(context, &mut mat) {
                mat.uuid = *id;
                context.result = Some(mat);

                return true;
            }
        }

        false
    }
}
