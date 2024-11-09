use crate::cir::{CirProgram, Translate};
use crate::context::{Context, Match};
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
    matchers: BTreeMap<MatcherKey, CirProgram>,
    pub fields: HashMap<String, usize>,
}

impl<'a> Router<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            matchers: BTreeMap::new(),
            fields: HashMap::new(),
        }
    }

    pub fn add_matcher(&mut self, priority: usize, uuid: Uuid, atc: &str) -> Result<(), String> {
        let key = MatcherKey(priority, uuid);

        if self.matchers.contains_key(&key) {
            return Err("UUID already exists".to_string());
        }

        let ast = parse(atc).map_err(|e| e.to_string())?;
        ast.validate(self.schema)?;
        let cir = ast.translate();
        cir.add_to_counter(&mut self.fields);
        assert!(self.matchers.insert(key, cir).is_none());

        Ok(())
    }

    pub fn remove_matcher(&mut self, priority: usize, uuid: Uuid) -> bool {
        let key = MatcherKey(priority, uuid);

        if let Some(cir) = self.matchers.remove(&key) {
            cir.remove_from_counter(&mut self.fields);
            return true;
        }

        false
    }

    pub fn execute(&self, context: &mut Context, collect_all: bool) -> bool {
        let mut matched = false;
        for (MatcherKey(_, id), m) in self.matchers.iter().rev() {
            let mut mat = Match::new();
            if m.execute(context, &mut mat) {
                matched = true;
                mat.uuid = *id;
                if collect_all {
                    context.results.push(mat);
                }
                else {
                  context.result = Some(mat);
                  return true;
                }
            }
        }

        return matched;
    }
}
