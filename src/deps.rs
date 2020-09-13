use std::{collections::HashMap, path::PathBuf};

use crate::{
    error::{Error, Result},
    target::Target,
};

use daggy::{Dag as Graph, NodeIndex};

enum DepOrigin {
    Output(NodeIndex),
    LeafFile,
}

pub struct DependencyGraph {
    dep_to_origin_map: HashMap<PathBuf, DepOrigin>,
    graph: Graph<Target, ()>,
}

impl DependencyGraph {
    pub fn construct(targets: Vec<Target>) -> Result<Self> {
        let mut graph = Graph::new();
        let mut dep_to_origin_map = HashMap::<PathBuf, _>::new();

        targets.iter().for_each(|target| {
            target.deps.iter().for_each(|dep| {
                // initially assume every dependency is a plain file
                dep_to_origin_map.insert(dep.clone(), DepOrigin::LeafFile);
            });
        });

        targets
            .into_iter()
            .map(|target| -> Result<_> {
                let id = graph.add_node(target);

                // make each output name point to the
                // corresponding target
                match dep_to_origin_map
                    .insert(graph[id].output.clone(), DepOrigin::Output(id))
                {
                    Some(DepOrigin::Output(_colliding_id)) =>
                        Err(Error::DuplicateOutput),
                    _ => Ok(()),
                }
            })
            .collect::<Result<()>>()?;

        graph
            .graph()
            .node_indices()
            .map(|id| {
                // add edges from each target to its dependencies

                let deps_indices = graph[id]
                    .deps
                    .iter()
                    .filter_map(|dep| {
                        let origin =
                            dep_to_origin_map.get(dep).expect("no dependency");

                        match origin {
                            DepOrigin::Output(ix) => Some(*ix),
                            DepOrigin::LeafFile => None,
                        }
                    })
                    .collect::<Vec<NodeIndex>>(); // collecting in order to drop the reference to the graph

                deps_indices
                    .iter()
                    .map(|dep_id| {
                        graph
                            .add_edge(id, *dep_id, ())
                            .map(|_| ())
                            .map_err(|_| Error::DependencyCycle)
                    })
                    .collect::<Result<_>>()
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

    use daggy::{petgraph, *};
    use petgraph::prelude::{Direction, EdgeRef, Graph};

    pub fn generate_depth_map<N, E>(
        graph: &Graph<N, E>,
        target: NodeIndex,
    ) -> HashMap<NodeIndex, usize> {
        let mut depth_map: HashMap<NodeIndex, usize> = HashMap::new();
        let mut current_depth = 0;
        let mut queue: VecDeque<Vec<_>> = VecDeque::new();
        queue.push_front(vec![target]);

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
