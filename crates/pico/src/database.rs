use std::{any::Any, num::NonZeroUsize};

use lru::LruCache;

use crate::{
    dyn_eq::DynEq,
    node::{Dependency, DerivedNode, NodeId, NodeKind, SourceNode},
    params::ParamId,
};

#[derive(Debug)]
pub struct Database {
    pub current_epoch: u64,
    pub dependency_stack: Vec<Vec<(u64, Dependency)>>,
    pub nodes: LruCache<NodeId, DerivedNode>,
    pub sources: LruCache<NodeId, SourceNode>,
    pub values: LruCache<NodeId, Box<dyn DynEq>>,
    pub params: LruCache<ParamId, Box<dyn Any>>,
}

impl Database {
    pub fn new(cache_size: usize) -> Self {
        let cap = NonZeroUsize::new(cache_size).unwrap();
        Self {
            current_epoch: 0,
            dependency_stack: vec![],
            nodes: LruCache::new(cap),
            sources: LruCache::new(cap),
            params: LruCache::new(cap),
            values: LruCache::new(cap),
        }
    }

    #[allow(clippy::map_entry)]
    pub fn memo(
        &mut self,
        static_key: &'static str,
        param_id: ParamId,
        inner_fn: fn(&mut Database, ParamId) -> Box<dyn DynEq>,
    ) -> NodeId {
        let node_id = NodeId::derived(static_key, param_id);
        let time_calculated = if self.nodes.contains(&node_id) && self.values.contains(&node_id) {
            if self.any_dependency_changed(&node_id) {
                let (new_value, dependencies, time_calculated) =
                    self.call_inner_fn_and_collect_dependencies(param_id, inner_fn);
                let node = self
                    .nodes
                    .get_mut(&node_id)
                    .expect("node should exist. This is indicative of a bug in Isograph.");
                if let Some(value) = self.values.get_mut(&node_id) {
                    if *value != new_value {
                        *value = new_value
                    }
                } else {
                    self.values.put(node_id, new_value);
                }
                node.dependencies = dependencies;
                node.time_calculated = time_calculated;
                node.time_verified = self.current_epoch;
                time_calculated
            } else {
                self.nodes
                    .get(&node_id)
                    .expect("node should exist. This is indicative of a bug in Isograph.")
                    .time_calculated
            }
        } else {
            let (value, dependencies, time_calculated) =
                self.call_inner_fn_and_collect_dependencies(param_id, inner_fn);
            self.nodes.put(
                node_id,
                DerivedNode {
                    time_verified: self.current_epoch,
                    time_calculated,
                    dependencies,
                    inner_fn,
                },
            );
            self.values.put(node_id, value);
            time_calculated
        };

        self.register_dependency(node_id, time_calculated);
        node_id
    }

    fn any_dependency_changed(&mut self, node_id: &NodeId) -> bool {
        let dependencies = self
            .nodes
            .get(node_id)
            .expect("node should exist. This is indicative of a bug in Isograph.")
            .dependencies
            .iter()
            .filter_map(|dep| {
                if dep.time_verified_or_calculated != self.current_epoch {
                    Some((dep.node_to, dep.time_verified_or_calculated))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        dependencies
            .into_iter()
            .any(|(dep_node_id, time_verified)| match dep_node_id.kind {
                NodeKind::Source => self.source_node_changed(&dep_node_id, time_verified),
                NodeKind::Derived => self.derived_node_changed(&dep_node_id),
            })
    }

    fn source_node_changed(&mut self, node_id: &NodeId, time_verified: u64) -> bool {
        match self.sources.get(node_id) {
            Some(source) => source.time_calculated > time_verified,
            None => true,
        }
    }

    fn derived_node_changed(&mut self, node_id: &NodeId) -> bool {
        if !self.params.contains(&node_id.param_id) {
            return true;
        }
        let new_value = (self.nodes.get(node_id).unwrap().inner_fn)(self, node_id.param_id);
        match self.values.get(node_id) {
            Some(value) => *value != new_value,
            None => true,
        }
    }

    fn call_inner_fn_and_collect_dependencies(
        &mut self,
        param_id: ParamId,
        inner_fn: impl Fn(&mut Database, ParamId) -> Box<dyn DynEq>,
    ) -> (
        Box<dyn DynEq>,  /* value */
        Vec<Dependency>, /* dependencies */
        u64,             /* time_calculated */
    ) {
        self.dependency_stack.push(vec![]);
        let value = inner_fn(self, param_id);
        let registred_dependencies = self
            .dependency_stack
            .pop()
            .expect("dependency stack to not be empty. This is indicative of a bug in Isograph.");

        let (dependencies, time_calculated) = registred_dependencies.into_iter().fold(
            (vec![], 0),
            |(mut deps, mut max_time_calculated), (time_calculated, dep)| {
                deps.push(dep);
                max_time_calculated = std::cmp::max(max_time_calculated, time_calculated);
                (deps, max_time_calculated)
            },
        );
        (value, dependencies, time_calculated)
    }

    pub fn register_dependency(&mut self, node_id: NodeId, time_calculated: u64) {
        if let Some(dependencies) = self.dependency_stack.last_mut() {
            dependencies.push((
                time_calculated,
                Dependency {
                    node_to: node_id,
                    time_verified_or_calculated: self.current_epoch,
                },
            ));
        }
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new(10000)
    }
}
