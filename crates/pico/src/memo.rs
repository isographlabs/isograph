use pico_core::{
    container::Container,
    database::Database,
    dyn_eq::DynEq,
    epoch::Epoch,
    key::Key,
    node::{Dependency, DerivedNode, NodeId, NodeKind},
    params::ParamId,
    storage::{Storage, StorageMut},
};

pub enum MemoState {
    Memoized,
    Computed,
}

#[allow(clippy::map_entry)]
pub fn memo<Db: Database>(
    db: &mut Db,
    node_id: NodeId,
    inner_fn: fn(&mut Db, ParamId) -> Box<dyn DynEq>,
) -> MemoState {
    let mut state = MemoState::Memoized;
    let current_epoch = db.current_epoch();
    let time_calculated = if db.storage().nodes().contains_key(&node_id)
        && db.storage().values().contains_key(&node_id)
    {
        if any_dependency_changed(db, &node_id, current_epoch) {
            let (new_value, dependencies, time_calculated) =
                call_inner_fn_and_collect_dependencies(db, node_id.param_id, inner_fn);
            if let Some(value) = db.storage_mut().values().get_mut(&node_id) {
                if *value != new_value {
                    *value = new_value;
                    state = MemoState::Computed;
                }
            } else {
                db.storage_mut().values().insert(node_id, new_value);
                state = MemoState::Computed;
            }
            if let Some(node) = db.storage_mut().nodes().get_mut(&node_id) {
                node.dependencies = dependencies;
                node.time_calculated = time_calculated;
                node.time_verified = current_epoch;
            }
            time_calculated
        } else {
            let node = db
                .storage_mut()
                .nodes()
                .get_mut(&node_id)
                .expect("node should exist. This is indicative of a bug in Pico.");
            node.time_verified = current_epoch;
            node.time_calculated
        }
    } else {
        let (value, dependencies, time_calculated) =
            call_inner_fn_and_collect_dependencies(db, node_id.param_id, inner_fn);
        db.storage_mut().nodes().insert(
            node_id,
            DerivedNode {
                time_verified: current_epoch,
                time_calculated,
                dependencies,
                inner_fn,
            },
        );
        db.storage_mut().values().insert(node_id, value);
        state = MemoState::Computed;
        time_calculated
    };
    register_dependency(
        db,
        NodeKind::Derived(node_id),
        time_calculated,
        current_epoch,
    );
    state
}

fn any_dependency_changed<Db: Database>(
    db: &mut Db,
    node_id: &NodeId,
    current_epoch: Epoch,
) -> bool {
    let dependencies = db
        .storage()
        .nodes()
        .get(node_id)
        .expect("node should exist. This is indicative of a bug in Pico.")
        .dependencies
        .iter()
        .filter_map(|dep| {
            if dep.time_verified_or_calculated != current_epoch {
                Some((dep.node_to, dep.time_verified_or_calculated))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    dependencies
        .into_iter()
        .any(|(dep_node, time_verified)| match dep_node {
            NodeKind::Source(key) => source_node_changed(db, &key, time_verified),
            NodeKind::Derived(dep_node_id) => derived_node_changed(db, dep_node_id),
        })
}

fn source_node_changed<Db: Database>(db: &Db, key: &Key, time_verified: Epoch) -> bool {
    match db.storage().sources().get(key) {
        Some(source) => source.time_calculated > time_verified,
        None => true,
    }
}

fn derived_node_changed<Db: Database>(db: &mut Db, node_id: NodeId) -> bool {
    let inner_fn = if let Some(node) = db.storage().nodes().get(&node_id) {
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
    Epoch,           /* time_calculated */
) {
    let (value, registered_dependencies) = with_dependency_tracking(db, param_id, inner_fn);
    let (dependencies, time_calculated) = registered_dependencies.into_iter().fold(
        (vec![], Epoch::new()),
        |(mut deps, mut max_time_calculated), (time_calculated, dep)| {
            deps.push(dep);
            max_time_calculated = std::cmp::max(max_time_calculated, time_calculated);
            (deps, max_time_calculated)
        },
    );
    (value, dependencies, time_calculated)
}

pub fn register_dependency<Db: Database>(
    db: &mut Db,
    node: NodeKind,
    time_calculated: Epoch,
    current_epoch: Epoch,
) {
    if let Some(dependencies) = db.storage_mut().dependency_stack().last_mut() {
        dependencies.push((
            time_calculated,
            Dependency {
                node_to: node,
                time_verified_or_calculated: current_epoch,
            },
        ));
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
