use crate::ast::Expression;
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::parse;
use crate::schema::Schema;
use crate::semantics::{FieldCounter, Validate};
use std::collections::HashMap;
use uuid::Uuid;

pub struct Router<'a> {
    schema: &'a Schema,
    matchers: HashMap<Uuid, Expression>,
    pub fields: HashMap<String, usize>,
}

impl<'a> Router<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            matchers: HashMap::new(),
            fields: HashMap::new(),
        }
    }

    pub fn add_matcher(&mut self, uuid: Uuid, atc: &str) -> Result<(), String> {
        if self.matchers.contains_key(&uuid) {
            return Err("UUID already exists".to_string());
        }

        let ast = parse(atc).map_err(|e| e.to_string())?;

        ast.validate(self.schema)?;
        ast.add_to_counter(&mut self.fields);

        assert!(self.matchers.insert(uuid, ast).is_none());

        Ok(())
    }

    pub fn remove_matcher(&mut self, uuid: &Uuid) -> bool {
        if let Some(ast) = self.matchers.remove(uuid) {
            ast.remove_from_counter(&mut self.fields);
            return true;
        }

        false
    }

    pub fn execute(&self, context: &mut Context) -> bool {
        let mut matched = false;

        for (id, m) in &self.matchers {
            let mut mat = Match::new();
            if m.execute(context, &mut mat) {
                mat.uuid = *id;
                matched = true;
                context.matches.push(mat);
            }
        }

        matched
    }
}
