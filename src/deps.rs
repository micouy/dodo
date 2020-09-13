use std::{
    collections::HashMap,
    convert::AsRef,
    path::{Path, PathBuf},
};

use crate::{
    error::{Error, Result},
    target::Target,
};

use daggy::{Dag as Graph, NodeIndex};

enum DepOrigin {
    Target(NodeIndex),
    LeafFile,
}

pub struct DependencyGraph {
    dep_to_origin_map: HashMap<PathBuf, DepOrigin>,
    graph: Graph<Target, ()>,
}

impl DependencyGraph {
    pub fn construct(targets: Vec<Target>) -> Result<Self> {
        let mut graph = Graph::new();
        let mut dep_to_origin_map: HashMap<PathBuf, _> = targets
            .iter()
            .map(|target| target.deps.iter())
            .flatten()
            .map(|dep| {
                // initially assume every dependency is a plain file
                (dep.clone(), DepOrigin::LeafFile)
            })
            .colect();

        // make each output name point to the
        // corresponding target
        targets
            .into_iter()
            .map(|target| {
                let id = graph.add_node(target);
                let returned_slot = dep_to_origin_map
                    .insert(graph[id].target.clone(), DepOrigin::Target(id));

                match returned_slot {
                    Some(DepOrigin::Target(_colliding_id)) =>
                        Err(Error::DuplicateTarget),
                    _ => Ok(()),
                }
            })
            .collect::<Result<()>>()?;

        // add edges from each target to its dependencies
        graph
            .graph()
            .node_indices()
            .map(|target_id| {
                util::connect_target_to_deps(
                    &mut graph,
                    &dep_to_origin_map,
                    target_id,
                )
            })
            .collect::<Result<_>>()?;

        Ok(Self {
            graph,
            dep_to_origin_map,
        })
    }
}

mod util {
    use std::collections::{HashMap, VecDeque};

    use super::*;

    use daggy::{petgraph, *};
    use petgraph::prelude::{Direction, EdgeRef, Graph};

    pub(super) fn connect_target_to_deps(
        graph: &mut Dag<Target, ()>,
        dep_to_origin_map: &HashMap<PathBuf, DepOrigin>,
        target_id: NodeIndex,
    ) -> Result<()> {
        // collecting indices to be able to drop
        // the reference to the graph
        let deps_ids = graph[target_id]
            .deps
            .iter()
            .filter_map(|dep| {
                let origin = dep_to_origin_map.get(dep).unwrap(); // unreachable

                match origin {
                    DepOrigin::Target(ix) => Some(*ix),
                    DepOrigin::LeafFile => None,
                }
            })
            .collect::<Vec<NodeIndex>>();

        // add edges
        deps_ids
            .iter()
            .map(|dep_id| {
                graph
                    .add_edge(target_id, *dep_id, ())
                    .map(|_| ())
                    .map_err(|_| Error::DependencyCycle)
            })
            .collect::<Result<_>>()
    }

    pub(super) fn generate_depth_map<N, E>(
        graph: &Graph<N, E>,
        target_id: NodeIndex,
    ) -> HashMap<NodeIndex, usize> {
        let mut depth_map: HashMap<NodeIndex, usize> = HashMap::new();
        let mut current_depth = 0;
        let mut queue: VecDeque<Vec<_>> = VecDeque::new();
        queue.push_front(vec![target_id]);

        while let Some(level) = queue.pop_front() {
            if level.is_empty() {
                break;
            }

            let mut level_queue = vec![];

            for current_node in level {
                // update current node's depth
                let _ = depth_map
                    .entry(current_node)
                    .and_modify(|depth| *depth = (*depth).max(current_depth))
                    .or_insert(current_depth);

                // add children to bfs queue
                let children = graph
                    // .graph()
                    .edges_directed(current_node, Direction::Outgoing)
                    .map(|edge| edge.target());
                level_queue.extend(children);
            }

            queue.push_back(level_queue);
            current_depth += 1;
        }

        depth_map
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use daggy::petgraph::graph::node_index as n;

    #[test]
    fn test_construct_dep_graph() {
        #[rustfmt::skip]
        let graph: Graph<(), ()> = Graph::from_edges(&[
            (0, 3), (0, 4),
            (1, 3), (1, 4), (1, 6),
            (2, 3), (2, 4),
            (3, 5), (3, 6), (3, 7),
            (4, 5), (4, 6), (4, 7),
            (7, 8),
            (8, 9),
        ]).unwrap();

        let target = NodeIndex::new(1); // target
        let depth_map = util::generate_depth_map(&graph.graph(), target);

        assert!(depth_map.get(&n(0)).is_none());
        assert!(depth_map.get(&n(2)).is_none());

        assert_eq!(depth_map[&n(1)], 0);
        assert_eq!(depth_map[&n(3)], 1);
        assert_eq!(depth_map[&n(4)], 1);
        assert_eq!(depth_map[&n(5)], 2);
        assert_eq!(depth_map[&n(6)], 2);
        assert_eq!(depth_map[&n(7)], 2);
        assert_eq!(depth_map[&n(8)], 3);
        assert_eq!(depth_map[&n(9)], 4);
    }
}
