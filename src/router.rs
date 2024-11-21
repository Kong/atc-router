use crate::ast::Value;
use crate::cir::{CirInstruction, CirProgram, Translate};
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::parser::parse;
use crate::schema::Schema;
use crate::semantics::{FieldCounter, Validate};
use regex::Regex;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use uuid::Uuid;

// use crate::ast::{Expression, LogicalExpression, Value};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct MatcherKey(usize, Uuid);

pub struct Router<'a> {
    schema: &'a Schema,
    matchers: BTreeMap<MatcherKey, CirProgram>,
    pub fields: HashMap<String, usize>,
    pub regex_cache: HashMap<String, Rc<Regex>>,
}

fn release_cache(cir: &CirProgram, router: &mut Router) {
    cir.instructions.iter().for_each(|instruction| {
        if let CirInstruction::Predicate(predicate) = instruction {
            if let Value::Regex(rc) = &predicate.rhs {
                if Rc::strong_count(rc) == 2 {
                    // about to be dropped and the only Rc left is in the map
                    router.regex_cache.remove(rc.as_str());
                }
            }
        }
    });
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
        let cir = ast.translate();
        cir.add_to_counter(&mut self.fields);
        assert!(self.matchers.insert(key, cir).is_none());

        Ok(())
    }

    pub fn remove_matcher(&mut self, priority: usize, uuid: Uuid) -> bool {
        let key = MatcherKey(priority, uuid);

        if let Some(cir) = self.matchers.remove(&key) {
            cir.remove_from_counter(&mut self.fields);
            release_cache(&cir, self);
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
