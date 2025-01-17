use pico_core::{
    container::Container,
    database::Database,
    dyn_eq::DynEq,
    epoch::Epoch,
    key::Key,
    node::{Dependency, DerivedNode, DerivedNodeId, NodeKind},
    params::ParamId,
    storage::{Storage, StorageMut},
};

pub enum MemoState {
    Memoized,
    Computed,
}

pub fn memo<Db: Database>(
    db: &mut Db,
    node_id: DerivedNodeId,
    inner_fn: fn(&mut Db, ParamId) -> Box<dyn DynEq>,
) -> MemoState {
    let current_epoch = db.current_epoch();
    let (time_updated, state) = if db.storage().derived_nodes().contains_key(&node_id) {
        if any_dependency_changed(db, node_id, current_epoch) {
            let mut state = MemoState::Memoized;
            let (value, dependencies, time_updated) =
                call_inner_fn_and_collect_dependencies(db, node_id.param_id, inner_fn);
            if let Some(node) = db.storage_mut().derived_nodes().get_mut(&node_id) {
                if node.value != value {
                    node.value = value;
                    state = MemoState::Computed;
                }
                node.dependencies = dependencies;
                node.time_updated = time_updated;
                node.time_verified = current_epoch;
            } else {
                db.storage_mut().derived_nodes().insert(
                    node_id,
                    DerivedNode {
                        time_verified: current_epoch,
                        time_updated,
                        dependencies,
                        inner_fn,
                        value,
                    },
                );
                state = MemoState::Computed;
            }
            (time_updated, state)
        } else {
            let node = db
                .storage_mut()
                .derived_nodes()
                .get_mut(&node_id)
                .expect("node should exist. This is indicative of a bug in Pico.");
            node.time_verified = current_epoch;
            (node.time_updated, MemoState::Memoized)
        }
    } else {
        let (value, dependencies, time_updated) =
            call_inner_fn_and_collect_dependencies(db, node_id.param_id, inner_fn);
        db.storage_mut().derived_nodes().insert(
            node_id,
            DerivedNode {
                time_verified: current_epoch,
                time_updated,
                dependencies,
                inner_fn,
                value,
            },
        );
        (time_updated, MemoState::Computed)
    };
    register_dependency_in_parent_memoized_fn(
        db,
        NodeKind::Derived(node_id),
        time_updated,
        current_epoch,
    );
    state
}

fn any_dependency_changed<Db: Database>(
    db: &mut Db,
    node_id: DerivedNodeId,
    current_epoch: Epoch,
) -> bool {
    let dependencies = db
        .storage()
        .derived_nodes()
        .get(&node_id)
        .expect("node should exist. This is indicative of a bug in Pico.")
        .dependencies
        .iter()
        .filter_map(|dep| {
            if dep.time_verified_or_updated != current_epoch {
                Some((dep.node_to, dep.time_verified_or_updated))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    dependencies
        .into_iter()
        .any(|(dep_node, time_verified)| match dep_node {
            NodeKind::Source(key) => source_node_changed_since(db, key, time_verified),
            NodeKind::Derived(dep_node_id) => derived_node_changed(db, dep_node_id),
        })
}

fn source_node_changed_since<Db: Database>(db: &Db, key: Key, since: Epoch) -> bool {
    match db.storage().source_nodes().get(&key) {
        Some(source) => source.time_updated > since,
        None => panic!("Source node not found. This indicates a bug in Pico."),
    }
}

fn derived_node_changed<Db: Database>(db: &mut Db, node_id: DerivedNodeId) -> bool {
    let inner_fn = if let Some(node) = db.storage().derived_nodes().get(&node_id) {
        node.inner_fn
    } else {
        return true;
    };
    if !db.storage().params().contains_key(&node_id.param_id) {
        return true;
    }
    let state = memo(db, node_id, inner_fn);
    matches!(state, MemoState::Computed)
}

fn call_inner_fn_and_collect_dependencies<Db: Database>(
    db: &mut Db,
    param_id: ParamId,
    inner_fn: fn(&mut Db, ParamId) -> Box<dyn DynEq>,
) -> (
    Box<dyn DynEq>,  /* value */
    Vec<Dependency>, /* dependencies */
    Epoch,           /* time_updated */
) {
    let (value, registered_dependencies) = with_dependency_tracking(db, param_id, inner_fn);
    let (dependencies, time_updated) = registered_dependencies.into_iter().fold(
        (vec![], Epoch::new()),
        |(mut deps, mut max_time_updated), (time_updated, dep)| {
            deps.push(dep);
            max_time_updated = std::cmp::max(max_time_updated, time_updated);
            (deps, max_time_updated)
        },
    );
    (value, dependencies, time_updated)
}

pub fn register_dependency_in_parent_memoized_fn<Db: Database>(
    db: &mut Db,
    node: NodeKind,
    time_updated: Epoch,
    current_epoch: Epoch,
) {
    if let Some(dependencies) = db.storage_mut().dependency_stack().last_mut() {
        dependencies.push((
            time_updated,
            Dependency {
                node_to: node,
                time_verified_or_updated: current_epoch,
            },
        ));
    } else {
        // Dependency stack is empty for the outermost memoized function.
        // We don't need to register dependencies for it.
    }
}

fn with_dependency_tracking<Db>(
    db: &mut Db,
    param_id: ParamId,
    inner_fn: fn(&mut Db, ParamId) -> Box<dyn DynEq>,
) -> (Box<dyn DynEq>, Vec<(Epoch, Dependency)>)
where
    Db: Database,
{
    db.storage_mut().dependency_stack().push(vec![]);
    let value = inner_fn(db, param_id);
    let registered_dependencies = db
        .storage_mut()
        .dependency_stack()
        .pop()
        .expect("Dependency stack should not be empty. This indicates a bug in Pico.");
    (value, registered_dependencies)
}
