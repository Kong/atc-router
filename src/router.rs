use crate::ast::{
    BinaryOperator, Expression, LHSTransformations, LogicalExpression, Predicate, Value, LHS,
};
use crate::context::Context;
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

    pub fn execute(&self, context: &Context) -> bool {
        for m in self.matchers.values() {
            return m.execute(context);
        }

        false
    }
}
