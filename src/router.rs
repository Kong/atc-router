use crate::ast::{Expression, Value};
use crate::context::{Context, Match};
use crate::indexes::FieldIndex;
use crate::interpreter::Execute;
use crate::parse;
use crate::schema::Schema;
use crate::semantics::{FieldCounter, Validate};
use std::collections::HashSet;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone)]
pub struct MatcherKey(usize, Uuid);

pub struct Router<'a> {
    schema: &'a Schema,
    matchers: BTreeMap<MatcherKey, Expression>,
    pub fields: HashMap<String, usize>,
    path_index: FieldIndex,
}

impl<'a> Router<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            matchers: BTreeMap::new(),
            fields: HashMap::new(),
            path_index: FieldIndex::new(),
        }
    }

    pub fn add_matcher(&mut self, priority: usize, uuid: Uuid, atc: &str) -> Result<(), String> {
        let key = MatcherKey(priority, uuid);

        if self.matchers.contains_key(&key) {
            return Err("UUID already exists".to_string());
        }

        let ast = parse(atc).map_err(|e| e.to_string())?;

        ast.validate(self.schema)?;
        ast.add_to_counter(&mut self.fields);

        self.path_index.add_to_index(self.schema, &key, &ast);
        assert!(self.matchers.insert(key, ast).is_none());

        Ok(())
    }

    pub fn remove_matcher(&mut self, priority: usize, uuid: Uuid) -> bool {
        let key = MatcherKey(priority, uuid);

        if let Some(ast) = self.matchers.remove(&key) {
            ast.remove_from_counter(&mut self.fields);
            return true;
        }

        false
    }

    pub fn execute(&self, context: &mut Context) -> bool {
        let mut mat = Match::new();

        let reduced_set = if let Some(paths) = context.value_of("http.path") {
            if let Value::String(s) = &paths[0] {
                Some(self.path_index.reduce(s))
            } else {
                None
            }
        } else {
            None
        };

        for (MatcherKey(_, id), m) in self
            .matchers
            .iter()
            .rev()
            .filter(|matcher| reduced_set.as_ref().map_or(true, |s| s.contains(matcher.0)))
        {
            if m.execute(context, &mut mat) {
                mat.uuid = *id;
                context.result = Some(mat);

                return true;
            }

            mat.reset();
        }

        false
    }
}
