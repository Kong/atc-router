use crate::ast::{BinaryOperator, LogicalExpression};
use crate::router::MatcherKey;
use crate::schema::Schema;
use crate::{Expression, Value};
use radix_trie::{Trie, TrieCommon};
use std::collections::HashSet;

pub struct FieldIndex {
    prefix: Trie<String, HashSet<MatcherKey>>,
    no_prefix: HashSet<MatcherKey>,
}

impl FieldIndex {
    pub fn new() -> Self {
        FieldIndex {
            prefix: Trie::new(),
            no_prefix: HashSet::new(),
        }
    }

    fn add_to_index_helper(
        &mut self,
        schema: &Schema,
        key: &MatcherKey,
        expr: &Expression,
    ) -> bool {
        match expr {
            Expression::Logical(l) => match l.as_ref() {
                LogicalExpression::And(l, r) => {
                    self.add_to_index_helper(schema, key, l)
                        || self.add_to_index_helper(schema, key, r)
                }
                LogicalExpression::Or(l, r) => {
                    self.add_to_index_helper(schema, key, l)
                        || self.add_to_index_helper(schema, key, r)
                }
            },
            Expression::Predicate(p) => match p.op {
                BinaryOperator::Prefix => {
                    if let Value::String(s) = &p.rhs {
                        if let Some(keys) = self.prefix.get_mut(s) {
                            keys.insert(*key);
                        } else {
                            let mut set = HashSet::new();
                            set.insert(*key);

                            assert!(self.prefix.insert(s.to_string(), set).is_none());
                        }

                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
        }
    }

    pub fn add_to_index(&mut self, schema: &Schema, key: &MatcherKey, expr: &Expression) {
        if !self.add_to_index_helper(schema, key, expr) {
            self.no_prefix.insert(*key);
        }
    }

    pub fn reduce(&self, context_value: &str) -> HashSet<MatcherKey> {
        match self.prefix.get_ancestor(context_value) {
            None => self.no_prefix.clone(),
            Some(ans) => {
                let mut possibility = HashSet::new();
                possibility.extend(ans.value().unwrap().iter());

                let mut key = ans.key().unwrap();
                while let Some(ans) = self.prefix.get_ancestor(&key[..key.len() - 1]) {
                    possibility.extend(ans.value().unwrap().iter());
                    key = ans.key().unwrap();
                }

                self.no_prefix.union(&possibility).cloned().collect()
            }
        }
    }
}
