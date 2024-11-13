use intern::string_key::Intern;
use std::{
    any::Any,
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};
use string_key_newtype::string_key_newtype;

pub struct Database {
    pub current_epoch: usize,
    pub nodes: HashMap<NodeId, DerivedNode>,
    pub dependency_stack: Vec<Vec<(usize, Dependency)>>,
}

impl Database {
    pub fn calculate<TParam, TOutput>(
        &mut self,
        static_key: &'static str,
        param: TParam,
        inner_fn: impl Fn(&mut Database, TParam) -> TOutput + 'static,
    ) -> TOutput
    where
        TParam: Hash + Clone + 'static,
        TOutput: AnyEq + Eq + Clone,
    {
        let node_id = NodeId::new(static_key, &param);

        let (value, time_calculated) = if self.nodes.contains_key(&node_id) {
            // Collect dependent nodes ids that might be changed
            let maybe_changed = self
                .nodes
                .get(&node_id)
                .expect("node to exist")
                .dependencies
                .iter()
                .filter_map(|dependency| {
                    if dependency.time_verified_or_calculated != self.current_epoch {
                        Some(dependency.node_to)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            // Recalculate every changed dependency if needed and compare values
            let mut has_changed = false;
            for child_node_id in maybe_changed {
                // We are detaching a child node here: we cannot make sure that inside
                // the calculate function there is no mutable borrow of this child node happens
                let child_node = self.nodes.remove(&child_node_id).expect("node to exist");
                let new_value = (child_node.calculate)(self);
                if child_node.value != new_value {
                    has_changed = true;
                    break;
                }
                self.nodes.insert(child_node_id, child_node);
            }

            if has_changed {
                // If any dependency has been changed, recalculate our node
                let new_value = inner_fn(self, param);
                let node = self.nodes.get_mut(&node_id).expect("node to exist");
                let value = node
                    .value
                    .as_any()
                    .downcast_ref::<TOutput>()
                    .expect("should be of expected type");
                if *value != new_value {
                    node.value = Box::new(new_value.clone());
                }
                (new_value, 0)
            } else {
                let derived_node = self.nodes.get(&node_id).expect("node to exist");
                (
                    derived_node
                        .value
                        .as_any()
                        .downcast_ref::<TOutput>()
                        .expect("should be of expected type")
                        .clone(),
                    derived_node.time_calculated,
                )
            }
        } else {
            self.dependency_stack.push(vec![]);
            let value = inner_fn(self, param.clone());
            let dependencies_of_last_call_with_time_calculated = self
                .dependency_stack
                .pop()
                .expect("dependency stack to not be empty");

            let (dependencies_of_last_call, time_calculated) =
                dependencies_of_last_call_with_time_calculated
                    .into_iter()
                    .fold(
                        (vec![], 0),
                        |(mut deps_so_far, mut max_so_far), (time_calculated, dep)| {
                            deps_so_far.push(dep);
                            max_so_far = std::cmp::max(max_so_far, time_calculated);
                            (deps_so_far, max_so_far)
                        },
                    );

            let node = DerivedNode {
                value: Box::new(value.clone()),
                time_verified: self.current_epoch,
                time_calculated,
                dependencies: dependencies_of_last_call,
                calculate: Box::new(move |database| Box::new(inner_fn(database, param.clone()))),
            };
            self.nodes.insert(node_id, node);

            (value, time_calculated)
        };

        let last_dependency_stack = self
            .dependency_stack
            .last_mut()
            .expect("expected dependency stack to not be empty");
        last_dependency_stack.push((
            time_calculated,
            Dependency {
                node_to: node_id,
                time_verified_or_calculated: self.current_epoch,
            },
        ));

        value
    }
}

string_key_newtype!(HashString);

#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug)]
pub struct NodeId((&'static str, HashString));

impl NodeId {
    pub fn new<T: Hash>(key: &'static str, param: &T) -> Self {
        Self {
            0: (key, hash(param)),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Dependency {
    node_to: NodeId,
    time_verified_or_calculated: usize,
}

pub struct DerivedNode {
    pub value: Box<dyn AnyEq>,
    pub time_verified: usize,
    pub time_calculated: usize,
    pub dependencies: Vec<Dependency>,
    pub calculate: Box<dyn Fn(&mut Database) -> Box<dyn AnyEq>>,
}

fn hash<T: Hash>(t: &T) -> HashString {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish().to_string().intern().into()
}

pub trait AsDynCompare: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_dyn_compare(&self) -> &dyn DynCompare;
}

impl<T: Any + DynCompare> AsDynCompare for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_dyn_compare(&self) -> &dyn DynCompare {
        self
    }
}

pub trait DynCompare: AsDynCompare {
    fn dyn_eq(&self, other: &dyn DynCompare) -> bool;
}

impl<T: Any + PartialOrd> DynCompare for T {
    fn dyn_eq(&self, other: &dyn DynCompare) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Self>() {
            self == other
        } else {
            false
        }
    }
}

impl PartialEq<dyn DynCompare> for dyn DynCompare {
    fn eq(&self, other: &dyn DynCompare) -> bool {
        self.dyn_eq(other)
    }
}

pub trait AnyEq: DynCompare {}

impl Eq for dyn AnyEq {}

impl PartialEq<dyn AnyEq> for dyn AnyEq {
    fn eq(&self, other: &dyn AnyEq) -> bool {
        self.as_dyn_compare() == other.as_dyn_compare()
    }
}

impl PartialEq<&Self> for Box<dyn AnyEq> {
    fn eq(&self, other: &&Self) -> bool {
        <Self as PartialEq>::eq(self, *other)
    }
}
