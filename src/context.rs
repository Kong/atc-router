use crate::ast::Value;
use crate::schema::Schema;
use fnv::FnvHashMap;
use uuid::Uuid;

pub struct Match {
    pub uuid: Uuid,
    pub matches: FnvHashMap<String, Value>,
    pub captures: FnvHashMap<String, String>,
}

impl Match {
    pub fn new() -> Self {
        Match {
            uuid: Uuid::default(),
            matches: FnvHashMap::default(),
            captures: FnvHashMap::default(),
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
    values: FnvHashMap<String, Vec<Value>>,
    pub result: Option<Match>,
    pub results: Vec<Match>,
}

impl<'a> Context<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Context {
            schema,
            values: FnvHashMap::with_hasher(Default::default()),
            result: None,
            results: Vec::new(),
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
