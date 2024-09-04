use crate::ast::Expression;
use crate::ast::Route;
use crate::context::{Context, Match};
use crate::interpreter::Convert;
use crate::interpreter::Execute;
use crate::parser::parse;
use crate::schema::Schema;
use crate::semantics::{FieldCounter, Validate};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct MatcherKey(usize, Uuid);

pub struct Router<'a> {
    schema: &'a Schema,
    routes: BTreeMap<MatcherKey, Route>,
    pub fields: HashMap<String, usize>,
}

impl<'a> Router<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            routes: BTreeMap::new(),
            fields: HashMap::new(),
        }
    }

    pub fn convert_route(&mut self, ast: Expression) -> Route {
        let mut route = Route::new();
        ast.convert(&mut route);
        route
    }

    pub fn add_matcher(&mut self, priority: usize, uuid: Uuid, atc: &str) -> Result<(), String> {
        let key = MatcherKey(priority, uuid);

        if self.routes.contains_key(&key) {
            return Err("UUID already exists".to_string());
        }

        let route = self.convert_route(parse(atc).map_err(|e| e.to_string())?);
        route.validate(self.schema)?;
        route.add_to_counter(&mut self.fields);
        assert!(self.routes.insert(key, route).is_none());

        Ok(())
    }

    pub fn remove_matcher(&mut self, priority: usize, uuid: Uuid) -> bool {
        let key = MatcherKey(priority, uuid);

        if let Some(route) = self.routes.remove(&key) {
            route.remove_from_counter(&mut self.fields);
            return true;
        }

        false
    }

    pub fn execute(&self, context: &mut Context) -> bool {
        for (MatcherKey(_, id), m) in self.routes.iter().rev() {
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
