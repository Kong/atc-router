use crate::{ast::Value, router::Router};
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
    router: &'a Router<'a>,
    values: Vec<Option<Vec<Value>>>,
    pub result: Option<Match>,
}

impl<'a> Context<'a> {
    pub fn new(router: &'a Router) -> Self {
        Context {
            router,
            values: vec![None; router.fields.list.len()],
            result: None,
        }
    }

    pub fn add_value(&mut self, field: &str, value: Value) {
        if &value.my_type() != self.router.schema().type_of(field).unwrap() {
            panic!("value provided does not match schema");
        }
        if let Some(index) = self.router.fields.map.get(field) {
            if let Some(v) = &mut self.values[*index] {
                v.push(value);
            } else {
                self.values[*index] = Some(vec![value]);
            }
        }
    }

    pub fn add_value_by_index(&mut self, index: usize, value: Value) {
        if index >= self.values.len() {
            panic!(
                "value provided does not match schema: index {}, max fields count {}",
                index,
                self.values.len()
            );
        }

        if let Some(v) = &mut self.values[index] {
            v.push(value);
        } else {
            self.values[index] = Some(vec![value]);
        }
    }

    pub fn value_of(&self, index: usize) -> Option<&[Value]> {
        if !self.values.is_empty() && self.values[index].is_some() {
            Some(self.values[index].as_ref().unwrap().as_slice())
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        let len = self.values.len();
        // reserve the capacity of values for reuse, avoid re-alloc
        self.values.clear();
        self.values.resize_with(len, Default::default);
        self.result = None;
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{Type, Value};
    use crate::context::Context;
    use crate::router::Router;
    use crate::schema::Schema;
    use uuid::Uuid;

    fn setup_matcher(r: &mut Router) -> usize {
        let fields_cnt = 3;
        for i in 0..fields_cnt {
            let id: Uuid = Uuid::new_v4();
            let exp = format!(r#"http.path.segments.{} == "/bar""#, i.to_string());
            let pri = i;
            assert!(r.add_matcher(pri, id, exp.as_str()).is_ok());
        }
        fields_cnt
    }

    #[test]
    fn test_context() {
        let mut s = Schema::default();
        s.add_field("http.path.segments.*", Type::String);
        let mut r = Router::new(&s);
        let fields_cnt = setup_matcher(&mut r);

        let mut ctx = Context::new(&r);
        assert!(ctx.values.len() == fields_cnt);
        assert_eq!(ctx.values, vec![None; fields_cnt]);
        // access value with out of bound index
        assert_eq!(ctx.value_of(0), None);

        // add value in bound
        ctx.add_value("http.path.segments.1", Value::String("foo".to_string()));
        assert_eq!(ctx.value_of(0), None);
        assert_eq!(ctx.value_of(1).unwrap().len(), 1);
        assert_eq!(
            ctx.value_of(1).unwrap(),
            vec![Value::String("foo".to_string())].as_slice()
        );

        // reset context keeps values capacity with all None
        ctx.reset();
        assert!(ctx.values.len() == fields_cnt);
        assert_eq!(ctx.values, vec![None; fields_cnt]);

        // reuse this context
        ctx.add_value("http.path.segments.0", Value::String("bar".to_string()));
        ctx.add_value("http.path.segments.0", Value::String("foo".to_string()));
        assert!(ctx.values.len() == fields_cnt);
        assert_eq!(ctx.value_of(0).unwrap().len(), 2);
        assert_eq!(
            ctx.value_of(0).unwrap(),
            vec![
                Value::String("bar".to_string()),
                Value::String("foo".to_string())
            ]
            .as_slice()
        );
    }

    #[test]
    fn test_context_by_index() {
        let mut s = Schema::default();
        s.add_field("http.path.segments.*", Type::String);
        let mut r = Router::new(&s);
        let fields_cnt = setup_matcher(&mut r);

        let mut ctx = Context::new(&r);
        assert!(ctx.values.len() == fields_cnt);
        assert_eq!(ctx.values, vec![None; fields_cnt]);
        // access value with out of bound index
        assert_eq!(ctx.value_of(0), None);

        // add value in bound
        ctx.add_value_by_index(1, Value::String("foo".to_string()));
        assert_eq!(ctx.value_of(0), None);
        assert_eq!(ctx.value_of(1).unwrap().len(), 1);
        assert_eq!(
            ctx.value_of(1).unwrap(),
            vec![Value::String("foo".to_string())].as_slice()
        );

        // reset context keeps values capacity with all None
        ctx.reset();
        assert!(ctx.values.len() == fields_cnt);
        assert_eq!(ctx.values, vec![None; fields_cnt]);

        // reuse this context
        ctx.add_value_by_index(0, Value::String("bar".to_string()));
        ctx.add_value_by_index(0, Value::String("foo".to_string()));
        assert!(ctx.values.len() == fields_cnt);
        assert_eq!(ctx.value_of(0).unwrap().len(), 2);
        assert_eq!(
            ctx.value_of(0).unwrap(),
            vec![
                Value::String("bar".to_string()),
                Value::String("foo".to_string())
            ]
            .as_slice()
        );
    }
}
