use crate::ast::Value;
use crate::schema::Schema;
use std::collections::HashMap;

pub struct Context<'a, 'v> {
    schema: &'a Schema,
    values: HashMap<String, Value>,
    prefix: Option<&'v str>,
}

impl<'a, 'v> Context<'a, 'v> {
    pub fn new(schema: &'a Schema) -> Self {
        Context {
            schema,
            values: HashMap::new(),
            prefix: None,
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
