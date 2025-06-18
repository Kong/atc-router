use crate::ast::Value;
use crate::schema::Schema;
use fnv::FnvHashMap;
use rand::Rng;
use uuid::Uuid;

#[derive(Debug)]
pub struct Match {
    pub uuid: Uuid,
    pub matches: FnvHashMap<String, Value>,
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
}

impl Default for Match {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Context<'a> {
    schema: &'a Schema,
    values: FnvHashMap<String, Vec<Value>>,
    values_from: FnvHashMap<String, Vec<Box<dyn Fn() -> Value>>>,
    pub result: Option<Match>,
}

impl std::fmt::Debug for Context<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("schema", &self.schema)
            .field("values", &self.values)
            .field("result", &self.result)
            .finish()
    }
}

impl<'a> Context<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        let mut ctx = Context {
            schema,
            values: FnvHashMap::with_hasher(Default::default()),
            values_from: FnvHashMap::with_hasher(Default::default()),
            result: None,
        };

        // Initialize the context with default values
        ctx.add_value_from("random()", || {
            let mut rng = rand::rng();
            Value::Int(rng.random_range(0..=100))
        });

        ctx
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

    pub fn add_value_from(&mut self, field: &str, fun: impl Fn() -> Value + 'static) {
        self.values_from
            .entry(field.to_string())
            .or_default()
            .push(Box::new(fun));
    }

    pub fn value_of(&self, field: &str) -> Option<Vec<Value>> {
        let static_values = self.values.get(field).map(|v| v.as_slice());

        let dynamic_values = self
            .values_from
            .get(field)
            .map(|from| from.iter().map(|f| f()).collect::<Vec<_>>());

        match (static_values, dynamic_values) {
            (None, None) => None,
            (Some(sv), None) => Some(sv.to_vec()),
            (None, Some(dv)) => Some(dv),
            (Some(sv), Some(dv)) => Some([sv, dv.as_slice()].concat()),
        }
    }

    pub fn reset(&mut self) {
        self.values.clear();
        self.result = None;
    }
}
