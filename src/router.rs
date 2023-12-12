use crate::ast::Expression;
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::parser::parse;
use crate::schema::Schema;
use crate::semantics::{FieldCounter, Validate};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct MatcherKey(usize, Uuid);

pub struct FieldInfo {
    pub count: usize,
    pub index: usize,
}

pub struct Router<'a> {
    pub schema: &'a Schema,
    matchers: BTreeMap<MatcherKey, Expression>,
    pub fields: HashMap<String, FieldInfo>,
    // we can use bitmap, but I don't think it will make much difference with the expected number of fields
    pub field_index: Vec<bool>,
    field_index_first_free: usize,
}

impl Default for FieldInfo {
    fn default() -> Self {
        Self { index: 0, count: 1 }
    }
}

impl<'a> Router<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            matchers: BTreeMap::new(),
            fields: HashMap::new(),
            field_index: Vec::new(),
            field_index_first_free: 0,
        }
    }

    pub fn get_field_index(&self, field_name: &str) -> Option<usize> {
        self.fields.get(field_name).map(|i| i.index)
    }

    // returns field index which lookup of this field should be using later
    pub fn acquire_field_index(&mut self, field_name: &str) -> usize {
        self.fields
            .entry(field_name.to_string())
            .and_modify(|c| c.count += 1)
            .or_insert_with(|| {
                let mut info = FieldInfo::default();
                // find the next free index
                info.index = self.field_index_first_free;

                if self.field_index.len() <= self.field_index_first_free {
                    self.field_index
                        .resize(self.field_index_first_free + 1, false);
                }

                self.field_index[self.field_index_first_free] = true;
                self.field_index_first_free = self
                    .field_index
                    .iter()
                    .skip(self.field_index_first_free + 1)
                    .position(|&b| b)
                    .unwrap_or(self.field_index.len());
                info
            })
            .index
    }

    pub fn release_field_index(&mut self, field_name: &str) {
        let val = self.fields.get_mut(field_name).unwrap();
        val.count -= 1;

        if val.count == 0 {
            if val.index == self.field_index.len() - 1 {
                self.field_index.pop().unwrap();
            } else {
                self.field_index[val.index] = false;
                if val.index < self.field_index_first_free {
                    self.field_index_first_free = val.index;
                }
            }
            assert!(self.fields.remove(field_name).is_some());
        }
    }

    pub fn add_matcher(&mut self, priority: usize, uuid: Uuid, atc: &str) -> Result<(), String> {
        let key = MatcherKey(priority, uuid);

        if self.matchers.contains_key(&key) {
            return Err("UUID already exists".to_string());
        }

        let mut ast = parse(atc).map_err(|e| e.to_string())?;

        ast.validate(self.schema)?;
        ast.add_to_counter(self);

        assert!(self.matchers.insert(key, ast).is_none());

        Ok(())
    }

    pub fn remove_matcher(&mut self, priority: usize, uuid: Uuid) -> bool {
        let key = MatcherKey(priority, uuid);

        if let Some(ast) = self.matchers.remove(&key) {
            ast.remove_from_counter(self);
            return true;
        }

        false
    }

    pub fn execute(&self, context: &mut Context) -> bool {
        for (MatcherKey(_, id), m) in self.matchers.iter().rev() {
            let mut mat = Match::new(self.field_index.len());
            if m.execute(context, &mut mat) {
                mat.uuid = *id;
                context.result = Some(mat);

                return true;
            }
        }

        false
    }
}
