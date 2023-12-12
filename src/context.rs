use crate::ast::Value;
use crate::router::Router;
use std::collections::HashMap;
use uuid::Uuid;

pub struct Match {
    pub uuid: Uuid,
    pub matches: Vec<Option<Value>>,
    pub captures: HashMap<String, String>,
}

impl Match {
    pub fn new(matches_n: usize) -> Self {
        Match {
            uuid: Uuid::default(),
            matches: vec![None; matches_n],
            captures: HashMap::new(),
        }
    }
}

pub struct Context<'a> {
    router: &'a Router<'a>,
    values: Vec<Vec<Value>>,
    pub result: Option<Match>,
}

impl<'a> Context<'a> {
    pub fn new(router: &'a Router) -> Self {
        Context {
            router,
            values: vec![vec![]; router.field_index.len()],
            result: None,
        }
    }

    pub fn add_value(&mut self, field: &str, value: Value) {
        if &value.my_type() != self.router.schema.type_of(field).unwrap() {
            panic!("value provided does not match schema");
        }

        self.values[self
            .router
            .get_field_index(field)
            .expect("unneeded field: {}")]
        .push(value);
    }

    pub fn value_of(&self, field_index: usize) -> Option<&[Value]> {
        Some(self.values.get(field_index).unwrap())
    }
}
