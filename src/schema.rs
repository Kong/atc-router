use crate::ast::Type;
use std::collections::HashMap;

#[derive(Default)]
pub struct Schema {
    fields: HashMap<String, Type>,
}

impl Schema {
    #[inline]
    pub fn type_of(&self, field: &str) -> Option<&Type> {
        self.fields.get(field).or_else(|| {
            self.fields
                .get(&format!("{}.*", &field[..field.rfind('.')?]))
        })
    }

    #[inline]
    pub fn add_field(&mut self, field: &str, typ: Type) {
        self.fields.insert(field.to_string(), typ);
    }
}
