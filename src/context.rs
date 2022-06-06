use crate::ast::Value;
use crate::schema::Schema;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub struct Match {
    pub uuid: Uuid,
    pub prefix: Option<String>,
}

impl Match {
    pub fn new() -> Self {
        Match {
            uuid: Uuid::default(),
            prefix: None,
        }
    }
}

pub struct Context<'a> {
    schema: &'a Schema,
    values: HashMap<String, Value>,
    pub matches: Vec<Match>,
}

impl<'a> Context<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Context {
            schema,
            values: HashMap::new(),
            matches: Vec::new(),
        }
    }

    pub fn add_value(&mut self, field: &str, value: Value) {
        if &value.my_type() != self.schema.type_of(field).unwrap() {
            panic!("value provided does not match schema");
        }

        self.values.insert(field.to_string(), value);
    }

    pub fn value_of(&self, field: &str) -> &Value {
        self.values.get(field).unwrap()
    }
}
