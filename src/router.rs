use crate::cir::{CirProgram, Translate};
use crate::context::{Context, Match};
use crate::interpreter::Execute;
use crate::parser::parse;
use crate::schema::Schema;
use crate::semantics::{FieldCounter, Validate};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct MatcherKey(usize, Uuid);

pub struct Router<'a> {
    schema: &'a Schema,
    matchers: BTreeMap<MatcherKey, Expression>,
    pub fields: Vec<(String, usize)>, // fileds array of tuple(name, count)
    pub fields_map: HashMap<String, usize>, // field name -> index map
}

impl<'a> Router<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            matchers: BTreeMap::new(),
            fields: Vec::new(),
            fields_map: HashMap::new(),
        }
    }

    pub fn add_matcher(&mut self, priority: usize, uuid: Uuid, atc: &str) -> Result<(), String> {
        let key = MatcherKey(priority, uuid);

        if self.matchers.contains_key(&key) {
            return Err("UUID already exists".to_string());
        }
        // lhs's index maybe changed in `ast.add_to_counter`
        let mut ast = parse(atc).map_err(|e| e.to_string())?;

        ast.validate(self.schema)?;
        ast.add_to_counter(&mut self.fields, &mut self.fields_map);

        assert!(self.matchers.insert(key, ast).is_none());

        Ok(())
    }

    pub fn remove_matcher(&mut self, priority: usize, uuid: Uuid) -> bool {
        let key = MatcherKey(priority, uuid);

        if let Some(mut ast) = self.matchers.remove(&key) {
            let fields_cnt = self.fields.len();
            ast.remove_from_counter(&mut self.fields, &mut self.fields_map);
            // if fields array changed, we need to reindex lhs in matchers
            if self.fields.len() != fields_cnt {
                self.reindexing_matchers();
            }
            return true;
        }

        false
    }

    pub fn reindexing_matchers(&mut self) {
        for (_, m) in self.matchers.iter_mut() {
            m.fix_lhs_index(&self.fields_map);
        } 
    }

    pub fn execute(&self, context: &mut Context) -> bool {
        for (MatcherKey(_, id), m) in self.matchers.iter().rev() {
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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, net::{IpAddr, Ipv4Addr}};

    use uuid::Uuid;

    use crate::{ast::{Expression, LogicalExpression, Type, Value}, context::Context, router::Router, schema::Schema};

    type FieldsType = Vec<(String, usize)>;
    type ContextValues<'a> = HashMap<&'a str, Value>;

    fn setup_matcher(r: &mut Router, priority: usize, expression: &str) -> (Uuid, usize) {
        let id = Uuid::new_v4();
        r.add_matcher(priority, id, expression).ok().expect("failed to addd matcher");
        (id, priority)
    }

    fn init_context(fields: &FieldsType, ctx_values: &ContextValues) -> Context {
        let mut ctx = Context::new(fields.len());
        for (i, v) in fields.iter().enumerate() {
            let key = &v.0;
            if ctx_values.contains_key(key.as_str()) {
                ctx.add_value(i, ctx_values.get(key.as_str()).unwrap().clone());
            }
        }
        ctx
    }

    fn is_index_match(e: &Expression, rt: &Router) -> bool {
        match e {
            Expression::Logical(l) => match l.as_ref() {
                LogicalExpression::And(l, r) | LogicalExpression::Or(l, r) => {
                    is_index_match(l, rt) && is_index_match(r, rt)
                }
                LogicalExpression::Not(r) => {
                    is_index_match(r, rt)
                }
            }
            Expression::Predicate(p) => {
                rt.fields[p.lhs.index].0 == p.lhs.var_name && *rt.fields_map.get(&p.lhs.var_name).unwrap() == p.lhs.index
            }
        }
    }

    fn validate_index(r: &Router) -> bool {
        for (_, e) in r.matchers.iter() {
            if !is_index_match(e, r) {
                return false;
            }
        }       
        true
    }
    
    #[test]
    fn test_router_execution() {
        // init schema
        let mut s = Schema::default();
        s.add_field("http.host", Type::String);
        s.add_field("net.dst.port", Type::Int);
        s.add_field("net.src.ip", Type::IpAddr);

        // init router
        let mut r = Router::new(&s);
        assert!(r.fields.len() == 0);
        assert!(validate_index(&r));
        
        // add matchers
        let (id_0, pri_0) = setup_matcher(&mut r, 99, r#"http.host == "example.com""#);
        let (id_1, pri_1) = setup_matcher(&mut r, 98, r#"net.dst.port == 8443 || net.dst.port == 443"#);
        let (id_2, pri_2) = setup_matcher(&mut r, 97, r#"net.src.ip == 192.168.1.1"#);
        assert!(r.fields.len() == 3);
        assert!(validate_index(&r));

        // mock context values
        let mut ctx_values = HashMap::from([
            ("http.host", Value::String("example.com".to_string())),
            ("net.src.ip", Value::IpAddr(IpAddr::V4(Ipv4Addr::new(192,168,1,2))))
        ]);
        let mut ctx = init_context(&r.fields, &ctx_values);
        
        // match the first matcher
        let res = r.execute(&mut ctx);
        assert!(res);

        // delete matcher, no field match now
        r.remove_matcher(pri_0, id_0);
        assert!(r.fields.len() == 2);
        assert!(validate_index(&r));
        ctx = init_context(&r.fields, &ctx_values);
        assert!(!r.execute(&mut ctx));

        // context value change, match again
        *ctx_values.get_mut("net.src.ip").unwrap() = Value::IpAddr(IpAddr::V4(Ipv4Addr::new(192,168,1,1)));
        ctx = init_context(&r.fields, &ctx_values);
        assert!(r.execute(&mut ctx));

        // delete all matchers
        r.remove_matcher(pri_1, id_1);
        r.remove_matcher(pri_2, id_2);
        assert!(r.fields.len() == 0);
        assert!(validate_index(&r));
        ctx = init_context(&r.fields, &ctx_values);
        assert!(!r.execute(&mut ctx));

        // add a new matcher
        let (_, _) = setup_matcher(&mut r, 96, r#"net.src.ip == 192.168.1.1"#);
        assert!(r.fields.len() == 1);
        assert!(validate_index(&r));
        ctx = init_context(&r.fields, &ctx_values);
        assert!(r.execute(&mut ctx));
    }
}
