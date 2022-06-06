use crate::ast::{
    BinaryOperator, Expression, LHSTransformations, LogicalExpression, Predicate, Value, LHS,
};
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::parse;
use crate::schema::Schema;
use crate::semantics::Validate;
use crate::ATCParser;
use crate::Rule;
use pest::Parser;
use std::collections::HashMap;
use uuid::Uuid;

pub struct Router<'a> {
    schema: &'a Schema,
    matchers: HashMap<Uuid, Expression>,
}

impl<'a> Router<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            matchers: HashMap::new(),
        }
    }

    pub fn add_matcher(&mut self, uuid: Uuid, atc: &str) -> Result<(), String> {
        if self.matchers.contains_key(&uuid) {
            return Err("UUID already exists".to_string());
        }

        let ast = parse(atc).map_err(|e| e.to_string())?;

        ast.validate(&self.schema)?;

        assert!(self.matchers.insert(uuid, ast).is_none());

        Ok(())
    }

    pub fn remove_matcher(&mut self, uuid: &Uuid) -> bool {
        self.matchers.remove(&uuid).is_some()
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
