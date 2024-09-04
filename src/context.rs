use crate::ast::Value;
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

pub struct Context {
    values: Vec<Option<Vec<Value>>>,
    pub result: Option<Match>,
}

impl Context {
        pub fn new(fields_cnt: usize) -> Self {
        Context {
            values: vec![None; fields_cnt],
            result: None,
        }
    }

    pub fn add_value(&mut self, index: usize, value: Value) {
        if index >= self.values.len() {
            panic!("value provided does not match schema: index {}, max fields count {}", index, self.values.len());
        }

        if let Some(v) = &mut self.values[index] {
            v.push(value);
        } else {
            self.values[index] =  Some(vec![value]);
        }
    }

    pub fn value_of(&self, index: usize) -> Option<&[Value]> {
        if !self.values.is_empty() && self.values[index].is_some() { Some(self.values[index].as_ref().unwrap().as_slice()) } else {None}
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
    use crate::context::Context;
    use crate::ast::Value;
    #[test]
    fn test_context() {
        let fields_cnt = 3;
        let mut ctx = Context::new(fields_cnt);
        assert!(ctx.values.len() == fields_cnt);
        assert_eq!(ctx.values, vec![None; fields_cnt]);
        // access value with out of bound index
        assert_eq!(ctx.value_of(0), None);
        
        // add value in bound
        ctx.add_value(1, Value::String("foo".to_string()));
        assert_eq!(ctx.value_of(0), None);
        assert_eq!(ctx.value_of(1).unwrap().len(), 1);
        assert_eq!(ctx.value_of(1).unwrap(), vec![Value::String("foo".to_string())].as_slice());

        // reset context keeps values capacity with all None 
        ctx.reset();
        assert!(ctx.values.len() == fields_cnt);
        assert_eq!(ctx.values, vec![None; fields_cnt]);

        // reuse this context
        ctx.add_value(0, Value::String("bar".to_string()));
        ctx.add_value(0, Value::String("foo".to_string()));
        assert!(ctx.values.len() == fields_cnt);
        assert_eq!(ctx.value_of(0).unwrap().len(), 2);
        assert_eq!(ctx.value_of(0).unwrap(), vec![Value::String("bar".to_string()), Value::String("foo".to_string())].as_slice());
    }
}
