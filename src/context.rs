use crate::ast::{MatchedValue, Value};
use crate::schema::Schema;
use fnv::FnvHashMap;
use uuid::Uuid;

#[derive(Debug)]
pub struct Match {
    pub uuid: Uuid,
    pub matches: FnvHashMap<String, MatchedValue>,
    pub captures: FnvHashMap<String, String>,
}

impl Match {
    pub fn new() -> Self {
        Self {
            uuid: Uuid::default(),
            matches: FnvHashMap::default(),
            captures: FnvHashMap::default(),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.matches.clear();
        self.captures.clear();
    }
}

impl Default for Match {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Context<'a> {
    schema: &'a Schema,
    values: FnvHashMap<String, Vec<Value>>,
    pub result: Option<Match>,
}

impl<'a> Context<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            values: FnvHashMap::with_hasher(Default::default()),
            result: None,
        }
    }

    pub fn add_value(&mut self, field: &str, value: Value) -> Result<(), String> {
        let Some(expected) = self.schema.type_of(field) else {
            return Err(format!("field \"{field}\" is not defined in schema"));
        };

        let actual = value.my_type();
        if &actual != expected {
            return Err(format!(
                "value type {actual:?} does not match schema type {expected:?} for field \"{field}\""
            ));
        }

        self.values
            .entry(field.to_string())
            .or_default()
            .push(value);

        Ok(())
    }

    pub fn value_of(&self, field: &str) -> Option<&[Value]> {
        self.values.get(field).map(|v| v.as_slice())
    }

    pub fn reset(&mut self) {
        self.values.clear();
        self.result = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Type, Value};

    #[test]
    fn add_value_rejects_type_mismatch() {
        let mut schema = Schema::default();
        schema.add_field("tcp.port", Type::Int);
        let mut ctx = Context::new(&schema);

        let err = ctx
            .add_value("tcp.port", Value::String("80".to_string()))
            .unwrap_err();

        assert!(err.contains("does not match schema type"));
        assert!(ctx.value_of("tcp.port").is_none());
    }

    #[test]
    fn add_value_rejects_unknown_field() {
        let schema = Schema::default();
        let mut ctx = Context::new(&schema);

        let err = ctx.add_value("missing.field", Value::Int(1)).unwrap_err();

        assert!(err.contains("not defined in schema"));
    }

    #[test]
    fn add_value_accepts_matching_type() {
        let mut schema = Schema::default();
        schema.add_field("tcp.port", Type::Int);
        let mut ctx = Context::new(&schema);

        ctx.add_value("tcp.port", Value::Int(80)).unwrap();
        assert_eq!(ctx.value_of("tcp.port").unwrap(), &[Value::Int(80)]);
    }
}
