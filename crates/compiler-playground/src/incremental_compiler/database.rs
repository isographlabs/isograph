use intern::string_key::Intern;
use std::{
    any::Any,
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};
use string_key_newtype::string_key_newtype;

#[derive(Debug)]
pub struct Database {
    pub current_epoch: usize,
    pub nodes: HashMap<NodeId, Node>,
}

impl Database {
    pub fn create_view(&mut self) -> DatabaseView {
        DatabaseView {
            database: self,
            dependencies: vec![],
            max_time_calculated: 0,
        }
    }
}

pub struct DatabaseView<'db> {
    pub database: &'db mut Database,
    pub dependencies: Vec<Dependency>,
    pub max_time_calculated: usize,
}

impl<'db> DatabaseView<'db> {
    pub fn calculate<TParam, TOutput>(
        &'db mut self,
        static_key: &'static str,
        param: TParam,
        inner_fn: impl Fn(&mut DatabaseView, TParam) -> TOutput,
    ) -> TOutput
    where
        TParam: Hash,
        TOutput: Any + PartialEq + Clone,
    {
        let node_id = NodeId::new(static_key, &param);
        self.register_dependency(node_id);
        if self.database.nodes.contains_key(&node_id) {
            let maybe_changed =
                self.database
                    .nodes
                    .get(&node_id)
                    .map_or(false, |node| match node {
                        Node::Derived(derived_node) => {
                            derived_node.dependencies.iter().any(|dependency| {
                                dependency.time_verified != self.database.current_epoch
                            })
                        }
                        Node::Source(source_node) => {
                            source_node.time_calculated != self.database.current_epoch
                        }
                    });
            if maybe_changed {
                if let Some(node) = self.database.nodes.remove(&node_id) {
                    match node {
                        Node::Derived(mut derived_node) => {
                            let mut view = self.database.create_view();
                            let new_value = inner_fn(&mut view, param);
                            let value = derived_node
                                .value
                                .downcast_mut::<TOutput>()
                                .expect("should be of expected type");
                            derived_node.dependencies = view.dependencies;
                            derived_node.time_calculated = view.max_time_calculated;
                            derived_node.time_verified = view.database.current_epoch;
                            if new_value != *value {
                                *value = new_value.clone();
                            }
                            if derived_node.time_calculated > self.max_time_calculated {
                                self.max_time_calculated = derived_node.time_calculated;
                            }
                            self.database
                                .nodes
                                .insert(node_id, Node::Derived(derived_node));
                            new_value
                        }
                        Node::Source(mut source_node) => {
                            let value = {
                                let mut view = DatabaseView {
                                    database: self.database,
                                    dependencies: vec![],
                                    max_time_calculated: 0,
                                };
                                let new_value = inner_fn(&mut view, param);
                                let value = source_node
                                    .value
                                    .downcast_mut::<TOutput>()
                                    .expect("should be of expected type");
                                if new_value != *value {
                                    *value = new_value.clone();
                                }
                                source_node.time_calculated = self.database.current_epoch;
                                new_value
                            };
                            if source_node.time_calculated > self.max_time_calculated {
                                self.max_time_calculated = source_node.time_calculated;
                            }
                            self.database
                                .nodes
                                .insert(node_id, Node::Source(source_node));
                            value
                        }
                    }
                } else {
                    panic!("expected node to exist")
                }
            } else {
                // TODO: update time_verified
                match &self.database.nodes[&node_id] {
                    Node::Derived(derived_node) => derived_node
                        .value
                        .downcast_ref::<TOutput>()
                        .expect("should be of expected type")
                        .clone(),
                    Node::Source(source_node) => source_node
                        .value
                        .downcast_ref::<TOutput>()
                        .expect("should be of expected type")
                        .clone(),
                }
            }
        } else {
            let mut view = self.database.create_view();
            let value = inner_fn(&mut view, param);
            if view.dependencies.is_empty() {
                let node = SourceNode {
                    value: Box::new(value.clone()),
                    time_calculated: self.database.current_epoch,
                };
                self.database.nodes.insert(node_id, Node::Source(node));
            } else {
                let node = DerivedNode {
                    value: Box::new(value.clone()),
                    time_verified: view.database.current_epoch,
                    time_calculated: view.max_time_calculated,
                    dependencies: view.dependencies,
                };
                if node.time_calculated > self.max_time_calculated {
                    self.max_time_calculated = node.time_calculated;
                }
                self.database.nodes.insert(node_id, Node::Derived(node));
            }
            value
        }
    }

    fn register_dependency(&mut self, node_id: NodeId) {
        self.dependencies.push(Dependency {
            node_to: node_id,
            time_verified: self.database.current_epoch,
        });
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

#[derive(Clone, Copy, Debug)]
pub struct Dependency {
    node_to: NodeId,
    time_verified: usize,
}

#[derive(Debug)]
pub enum Node {
    Derived(DerivedNode),
    Source(SourceNode),
}

#[derive(Debug)]
pub struct DerivedNode {
    pub value: Box<dyn Any>,
    pub time_verified: usize,
    pub time_calculated: usize,
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug)]
pub struct SourceNode {
    pub value: Box<dyn Any>,
    pub time_calculated: usize,
}

fn hash<T: Hash>(t: &T) -> HashString {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish().to_string().intern().into()
}
