use crate::ast::Type;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Schema {
    fields: HashMap<String, Type>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normail_fields() {
        let mut s = Schema::default();

        s.add_field("str", Type::String);
        s.add_field("ip", Type::IpAddr);
        s.add_field("cidr", Type::IpCidr);
        s.add_field("r", Type::Regex);
        s.add_field("i", Type::Int);

        assert_eq!(s.type_of("str"), Some(&Type::String));
        assert_eq!(s.type_of("ip"), Some(&Type::IpAddr));
        assert_eq!(s.type_of("cidr"), Some(&Type::IpCidr));
        assert_eq!(s.type_of("r"), Some(&Type::Regex));
        assert_eq!(s.type_of("i"), Some(&Type::Int));

        assert_eq!(s.type_of("unknown"), None);
    }

    #[test]
    fn wildcard_fields() {
        let mut s = Schema::default();

        s.add_field("a.*", Type::String);

        assert_eq!(s.type_of("a.b"), Some(&Type::String));
        assert_eq!(s.type_of("a.xxx"), Some(&Type::String));

        assert_eq!(s.type_of("a.x.y"), None);
    }
}
