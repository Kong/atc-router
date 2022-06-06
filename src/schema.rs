use crate::ast::Type;
use std::collections::HashMap;

pub struct Schema {
    fields: HashMap<String, Type>,
}

impl Schema {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    pub fn type_of(&self, field: &str) -> Option<&Type> {
        self.fields.get(field)
    }

    pub fn add_field(&mut self, field: &str, typ: Type) {
        self.fields.insert(field.to_string(), typ);
    }
}
