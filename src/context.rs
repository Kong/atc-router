use crate::ast::Value;
use crate::schema::Schema;
use std::collections::HashMap;
use uuid::Uuid;

pub struct Match {
    pub uuid: Uuid,
    pub matches: HashMap<String, Value>,
    pub captures: HashMap<String, String>,
}

impl Match {
    pub fn new() -> Self {
        Match {
            uuid: Uuid::default(),
            matches: HashMap::new(),
            captures: HashMap::new(),
        }
    }
}

impl Default for Match {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Context<'a> {
    schema: &'a Schema,
    values: HashMap<String, Vec<Value>>,
    pub result: Option<Match>,
}

impl<'a> Context<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Context {
            schema,
            values: HashMap::new(),
            result: None,
        }
    }

    pub fn add_value(&mut self, field: &str, value: Value) {
        if &value.my_type() != self.schema.type_of(field).unwrap() {
            panic!("value provided does not match schemas");
        }

        self.values
            .entry(field.to_string())
            .or_default()
            .push(value);
    }

    pub fn value_of(&self, field: &str) -> Option<&[Value]> {
        self.values.get(field).map(|v| v.as_slice())
    }
}
