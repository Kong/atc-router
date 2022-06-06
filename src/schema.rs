use crate::ast::Type;
use std::collections::HashMap;

#[derive(Default)]
pub struct Schema {
    fields: HashMap<String, Type>,
}

impl Schema {
    pub fn type_of(&self, field: &str) -> Option<&Type> {
        self.fields.get(field)
    }

    pub fn add_field(&mut self, field: &str, typ: Type) {
        self.fields.insert(field.to_string(), typ);
    }
}
