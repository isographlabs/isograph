use std::collections::HashSet;

use boxcar::Vec as BoxcarVec;
use dashmap::DashMap;

use crate::{
    dependency::{Dependency, NodeKind},
    index::Index,
    DatabaseStorage, DerivedNode, DerivedNodeId, DerivedNodeRevision, ParamId,
};

impl DatabaseStorage {
    /// Run garbage collection, retaining retained_derived_node_ids (which represent
    /// top level function calls) and everything reachable from them.
    ///
    /// This will create a new values for `self.derived_nodes`, `self.params`,
    /// `self.param_id_to_index` and `self.derived_node_id_to_revision`.
    ///
    /// We do not garbage collect source nodes. Those are managed by the end user.
    pub fn run_garbage_collection(
        &mut self,
        retained_derived_node_ids: impl Iterator<Item = DerivedNodeId>,
    ) {
        let mut derived_node_id_queue = retained_derived_node_ids.collect::<Vec<_>>();

        // We need to keep track of nodes that we have already processed, since one top-level retained node
        // can be reachable from another, e.g. if the user called both capitalized_first_letter and first_letter.
        let mut processed_nodes = HashSet::new();
        let mut processed_params = HashSet::new();

        let new_params = BoxcarVec::new();
        let new_derived_nodes = BoxcarVec::new();
        let new_param_id_to_index = DashMap::new();
        let new_derived_node_id_to_revision = DashMap::new();

        'derived_node_id_queue: while let Some(derived_node_id) = derived_node_id_queue.pop() {
            if processed_nodes.contains(&derived_node_id) {
                continue 'derived_node_id_queue;
            }
            processed_nodes.insert(derived_node_id);

            let old_derived_node_revision = self
                .derived_node_id_to_revision
                .get(&derived_node_id)
                .expect("Expected revision to be present. This is indicative of a bug in Pico.");

            let old_derived_node = self
                .derived_nodes
                .get_mut(old_derived_node_revision.index.idx)
                .expect(
                    "Expected derived node to be present. This is indicative of a bug in Pico.",
                );

            add_dependencies_to_queue(
                &mut derived_node_id_queue,
                old_derived_node.dependencies.iter(),
            );

            // We do this to avoid cloning the inner value
            let derived_node_value = std::mem::replace(&mut old_derived_node.value, Box::new(()));

            let new_derived_node = DerivedNode {
                dependencies: old_derived_node.dependencies.clone(),
                inner_fn: old_derived_node.inner_fn,
                value: derived_node_value,
            };

            let new_index = Index::new(new_derived_nodes.push(new_derived_node));

            new_derived_node_id_to_revision.insert(
                derived_node_id,
                DerivedNodeRevision {
                    time_updated: old_derived_node_revision.time_updated,
                    time_verified: old_derived_node_revision.time_verified,
                    index: new_index,
                },
            );

            'param: for param_id in derived_node_id.params {
                if processed_params.contains(&param_id) {
                    continue 'param;
                }
                processed_params.insert(param_id);

                let old_param_index = self.param_id_to_index.get_mut(&param_id).expect(
                    "Expected param id to be present. This is indicative of a bug in Pico.",
                );

                let old_param = self
                    .params
                    .get_mut(old_param_index.idx)
                    .expect("Expected param to be present. This is indicative of a bug in Pico.");

                // Let's avoid cloning the param, as well
                let param = std::mem::replace(old_param, Box::new(()));

                let new_param_index: Index<ParamId> = Index::new(new_params.push(param));
                new_param_id_to_index.insert(param_id, new_param_index);
            }
        }

        self.params = new_params;
        self.derived_nodes = new_derived_nodes;
        self.param_id_to_index = new_param_id_to_index;
        self.derived_node_id_to_revision = new_derived_node_id_to_revision;
    }
}

fn add_dependencies_to_queue<'a>(
    derived_node_id_queue: &mut Vec<DerivedNodeId>,
    dependencies: impl Iterator<Item = &'a Dependency>,
) {
    for dependency in dependencies {
        match dependency.node_to {
            NodeKind::Source(_) => {}
            NodeKind::Derived(dependency_id) => {
                derived_node_id_queue.push(dependency_id);
            }
        }
    }
}
