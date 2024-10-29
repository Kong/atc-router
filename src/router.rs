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

pub struct Fields {
    pub list: Vec<Option<(String, usize)>>, // fileds list of tuple(name, count)
    pub slots: Vec<usize>,                  // slots in list to be reused
    pub map: HashMap<String, usize>,        // 'name' to 'list index' maping
}

pub struct Router<'a> {
    schema: &'a Schema,
    matchers: BTreeMap<MatcherKey, CirProgram>,
    pub fields: Fields,
}

impl<'a> Router<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            matchers: BTreeMap::new(),
            fields: Fields {
                list: Vec::new(),
                slots: Vec::new(),
                map: HashMap::new(),
            },
        }
    }

    pub fn add_matcher(&mut self, priority: usize, uuid: Uuid, atc: &str) -> Result<(), String> {
        let key = MatcherKey(priority, uuid);

        if self.matchers.contains_key(&key) {
            return Err("UUID already exists".to_string());
        }
        // lhs's index maybe changed in `ast.add_to_counter`
        let ast = parse(atc).map_err(|e| e.to_string())?;
        ast.validate(self.schema)?;
        let mut cir = ast.translate();
        cir.add_to_counter(&mut self.fields);
        assert!(self.matchers.insert(key, cir).is_none());

        Ok(())
    }

    pub fn remove_matcher(&mut self, priority: usize, uuid: Uuid) -> bool {
        let key = MatcherKey(priority, uuid);

        if let Some(mut ast) = self.matchers.remove(&key) {
            ast.remove_from_counter(&mut self.fields);
            return true;
        }

        false
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

    pub fn schema(&self) -> &Schema {
        &self.schema
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cmp::max,
        collections::HashMap,
        net::{IpAddr, Ipv4Addr},
    };

    use uuid::Uuid;

    use crate::{
        ast::{Type, Value},
        cir::{get_predicates, CirProgram},
        context::Context,
        router::Router,
        schema::Schema,
    };

    type ContextValues<'a> = HashMap<&'a str, Value>;

    fn setup_matcher(r: &mut Router, priority: usize, expression: &str) -> (Uuid, usize) {
        let id = Uuid::new_v4();
        r.add_matcher(priority, id, expression)
            .ok()
            .expect("failed to addd matcher");
        (id, priority)
    }

    fn init_context<'a>(r: &'a Router, ctx_values: &'a ContextValues<'a>) -> Context<'a> {
        let mut ctx = Context::new(r);
        for (i, v) in r.fields.list.iter().enumerate() {
            if v.is_none() {
                continue;
            }
            let key = &v.as_ref().unwrap().0;
            if ctx_values.contains_key(key.as_str()) {
                ctx.add_value_by_index(i, ctx_values.get(key.as_str()).unwrap().clone());
            }
        }
        ctx
    }

    fn is_index_match(cir: &CirProgram, rt: &Router) -> bool {
        let predicates = get_predicates(cir);
        for p in predicates {
            if rt.fields.list[p.lhs.index].as_ref().unwrap().0 == p.lhs.var_name
                && *rt.fields.map.get(&p.lhs.var_name).unwrap() == p.lhs.index
            {
                continue;
            }
            return false;
        }
        true
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
        assert!(r.fields.list.len() == 0);
        assert!(validate_index(&r));

        // add matchers
        let (id_0, pri_0) = setup_matcher(&mut r, 99, r#"http.host == "example.com""#);
        let (id_1, pri_1) =
            setup_matcher(&mut r, 98, r#"net.dst.port == 8443 || net.dst.port == 443"#);
        let (id_2, pri_2) = setup_matcher(&mut r, 97, r#"net.src.ip == 192.168.1.1"#);
        assert!(r.fields.list.len() == 3);
        assert!(r.fields.slots.len() == 0);
        assert!(validate_index(&r));

        // mock context values
        let mut ctx_values = HashMap::from([
            ("http.host", Value::String("example.com".to_string())),
            (
                "net.src.ip",
                Value::IpAddr(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2))),
            ),
        ]);
        let mut ctx = init_context(&r, &ctx_values);

        // match the first matcher
        let res = r.execute(&mut ctx);
        assert!(res);

        // delete matcher, no field match now
        r.remove_matcher(pri_0, id_0);
        assert!(r.fields.list.len() == 3);
        assert!(r.fields.slots.len() == 1);
        assert!(validate_index(&r));
        ctx = init_context(&r, &ctx_values);
        assert!(!r.execute(&mut ctx));

        // context value change, match again
        *ctx_values.get_mut("net.src.ip").unwrap() =
            Value::IpAddr(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));
        ctx = init_context(&r, &ctx_values);
        assert!(r.execute(&mut ctx));

        // delete all matchers
        r.remove_matcher(pri_1, id_1);
        r.remove_matcher(pri_2, id_2);
        assert!(r.fields.list.len() == 3);
        assert!(r.fields.slots.len() == 3);
        assert!(validate_index(&r));
        ctx = init_context(&r, &ctx_values);
        assert!(!r.execute(&mut ctx));

        // add a new matcher
        let (_, _) = setup_matcher(&mut r, 96, r#"net.src.ip == 192.168.1.1"#);
        assert!(r.fields.list.len() == 3);
        assert!(r.fields.slots.len() == 2);
        assert!(validate_index(&r));
        ctx = init_context(&r, &ctx_values);
        assert!(r.execute(&mut ctx));
    }

    #[test]
    fn test_fields_list() {
        let mut s = Schema::default();
        s.add_field("http.path.segments.*", Type::String);
        let mut r = Router::new(&s);
        let i_max = 1000;
        let mut ids = vec![];
        for i in 0..i_max {
            let id: Uuid = Uuid::new_v4();
            let exp = format!(r#"http.path.segments.{} == "/bar""#, i.to_string());
            let pri = i;
            assert!(r.add_matcher(pri, id, exp.as_str()).is_ok());
            assert!(r.fields.list.len() == i + 1);
            assert!(r.fields.slots.len() == 0);
            assert!(r.fields.map.len() == i + 1);
            ids.push((pri, id));
        }

        // delete 100 fields
        let mut valid_cnt = i_max;
        for (idx, id) in &ids[100..200] {
            let pri = idx;
            assert!(r.remove_matcher(*pri, *id));
            valid_cnt -= 1;
            assert!(r.fields.list.len() == i_max);
            assert!(r.fields.slots.len() == i_max - valid_cnt);
            assert!(r.fields.map.len() == valid_cnt);
        }

        // deleted fields leave None in fields list
        for i in 100..200 {
            assert!(r.fields.list[i] == None);
        }

        // adds 200 fields back
        let fields_len = r.fields.list.len();
        let mut slot_cnt = r.fields.slots.len();
        for i in 0..200 {
            let id: Uuid = Uuid::new_v4();
            let exp = format!(
                r#"http.path.segments.{} == "/bar""#,
                (i_max + i).to_string()
            );
            let pri = i;
            if slot_cnt > 0 {
                slot_cnt -= 1;
            }
            assert!(r.add_matcher(pri, id, exp.as_str()).is_ok());
            assert!(r.fields.list.len() == max(fields_len, r.fields.map.len()));
            assert!(r.fields.slots.len() == slot_cnt);
            assert!(r.fields.map.len() == r.fields.list.len() - slot_cnt);
        }

        // 100 slot deleted before should be reused
        for i in 100..200 {
            assert!(r.fields.list[i].is_some());
        }

        // 100 slot newly added should be valid
        for i in i_max..i_max + 100 {
            assert!(r.fields.list[i].is_some());
        }
    }
}
