use crate::ast::Value;
use crate::schema::Schema;
use uuid::Uuid;

type FastHashMap<K, V> = fnv::FnvHashMap<K, V>;

#[derive(Debug)]
pub struct Match {
    pub uuid: Uuid,
    pub matches: FastHashMap<String, Value>,
    pub captures: FastHashMap<String, String>,
}

impl Match {
    pub fn new() -> Self {
        Self {
            uuid: Uuid::default(),
            matches: FastHashMap::default(),
            captures: FastHashMap::default(),
        }
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
    values: FastHashMap<String, Vec<Value>>,
    pub result: Option<Match>,
}

impl<'a> Context<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Context {
            schema,
            values: FastHashMap::with_hasher(Default::default()),
            result: None,
        }
    }

    pub fn add_value(&mut self, field: &str, value: Value) {
        if &value.my_type() != self.schema.type_of(field).unwrap() {
            panic!("value provided does not match schema");
        }

        self.values
            .entry(field.to_string())
            .or_default()
            .push(value);
    }

    pub fn value_of(&self, field: &str) -> Option<&[Value]> {
        self.values.get(field).map(|v| v.as_slice())
    }

    pub fn reset(&mut self) {
        self.values.clear();
        self.result = None;
    }
}
