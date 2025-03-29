use ahash::AHashMap;

use crate::ast::Type;

#[derive(Debug, Default)]
pub struct Schema {
    fields: AHashMap<String, Type>,
}

impl Schema {
    pub fn type_of(&self, field: &str) -> Option<&Type> {
        self.fields.get(field).or_else(|| {
            self.fields
                .get(&format!("{}.*", &field[..field.rfind('.')?]))
        })
    }

    pub fn add_field(&mut self, field: &str, typ: Type) {
        self.fields.insert(field.to_string(), typ);
    }
}
