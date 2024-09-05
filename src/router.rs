use crate::ast::Expression;
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::linear::Lir;
use crate::linear::Translate;
use crate::parser::parse;
use crate::schema::Schema;
use crate::semantics::{FieldCounter, Validate};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct MatcherKey(usize, Uuid);

pub struct Router<'a> {
    schema: &'a Schema,
    lirs: BTreeMap<MatcherKey, Lir>,
    pub fields: HashMap<String, usize>,
}

impl<'a> Router<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            lirs: BTreeMap::new(),
            fields: HashMap::new(),
        }
    }

    pub fn translate_lir(&mut self, ast: Expression) -> Lir {
        let mut lir = Lir::new();
        ast.translate(&mut lir);
        lir
    }

    pub fn add_matcher(&mut self, priority: usize, uuid: Uuid, atc: &str) -> Result<(), String> {
        let key = MatcherKey(priority, uuid);

        if self.lirs.contains_key(&key) {
            return Err("UUID already exists".to_string());
        }

        let ast = parse(atc).map_err(|e| e.to_string())?;
        ast.validate(self.schema)?;
        let lir = self.translate_lir(ast);
        lir.add_to_counter(&mut self.fields);
        assert!(self.lirs.insert(key, lir).is_none());

        Ok(())
    }

    pub fn remove_matcher(&mut self, priority: usize, uuid: Uuid) -> bool {
        let key = MatcherKey(priority, uuid);

        if let Some(lir) = self.lirs.remove(&key) {
            lir.remove_from_counter(&mut self.fields);
            return true;
        }

        false
    }

    pub fn execute(&self, context: &mut Context) -> bool {
        for (MatcherKey(_, id), m) in self.lirs.iter().rev() {
            let mut mat = Match::new();
            if m.execute(context, &mut mat) {
                mat.uuid = *id;
                context.result = Some(mat);

                return true;
            }
        }

        false
    }
}
