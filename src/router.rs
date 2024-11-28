use crate::ast::Expression;
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::parser::parse;
use crate::schema::Schema;
use crate::semantics::{FieldCounter, Validate};
use std::cell::UnsafeCell;
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct MatcherKey(usize, Uuid);

pub struct Router<'a> {
    schema: &'a Schema,
    matchers: BTreeMap<MatcherKey, Expression>,
    pub fields: HashMap<String, usize>,
    pub add_matcher_duration: Duration,
    pub remove_matcher_duration: Duration,
    // Safety: Nginx is single-threaded, no need for synchronization
    pub execute_duration: UnsafeCell<Duration>,
}

impl<'a> Router<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            matchers: BTreeMap::new(),
            fields: HashMap::new(),
            add_matcher_duration: Duration::default(),
            remove_matcher_duration: Duration::default(),
            execute_duration: UnsafeCell::new(Duration::default()),
        }
    }

    pub fn add_matcher(&mut self, priority: usize, uuid: Uuid, atc: &str) -> Result<(), String> {
        let start = Instant::now();

        let key = MatcherKey(priority, uuid);

        if self.matchers.contains_key(&key) {
            return Err("UUID already exists".to_string());
        }

        let ast = parse(atc).map_err(|e| e.to_string())?;

        ast.validate(self.schema)?;
        ast.add_to_counter(&mut self.fields);

        assert!(self.matchers.insert(key, ast).is_none());

        self.add_matcher_duration += start.elapsed();
        Ok(())
    }

    pub fn remove_matcher(&mut self, priority: usize, uuid: Uuid) -> bool {
        let start = Instant::now();

        let key = MatcherKey(priority, uuid);

        if let Some(ast) = self.matchers.remove(&key) {
            ast.remove_from_counter(&mut self.fields);
            self.remove_matcher_duration += start.elapsed();
            return true;
        }

        self.remove_matcher_duration += start.elapsed();
        false
    }

    pub fn execute(&self, context: &mut Context) -> bool {
        let start = Instant::now();

        for (MatcherKey(_, id), m) in self.matchers.iter().rev() {
            let mut mat = Match::new();
            if m.execute(context, &mut mat) {
                mat.uuid = *id;
                context.result = Some(mat);

                let duration = start.elapsed();
                let execute_duration = unsafe { &mut *self.execute_duration.get() };
                *execute_duration += duration;

                return true;
            }
        }

        let duration = start.elapsed();
        let execute_duration = unsafe { &mut *self.execute_duration.get() };
        *execute_duration += duration;

        false
    }
}
