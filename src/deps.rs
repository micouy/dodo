use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use crate::{
    error::{Error, Result, UserError},
    target::Target,
    util::ResultIterator,
};

use daggy::{petgraph::visit::IntoNeighborsDirected, NodeIndex as Nx};

type DependencyDag = daggy::Dag<Node, ()>;
type Identifier = PathBuf;
type FileState = (); // TODO

#[derive(Debug)]
enum Node {
    Target(Target),
    NoRule(Identifier),
}

// TODO think of a better name
pub struct DependencyGraph {
    id_to_ix_map: HashMap<Identifier, Nx>,
    graph: DependencyDag,
}

impl DependencyGraph {
    pub fn construct(targets: Vec<Target>) -> Result<Self> {
        let mut graph = DependencyDag::new();
        let mut id_to_ix_map = HashMap::new();

        // add target nodes
        targets
            .iter()
            .cloned()
            .map(|target| {
                util::add_target_node(&mut graph, &mut id_to_ix_map, target)
            })
            .collect::<Result<_>>()?;

        // add left dependency nodes - leaf nodes representing actual files
        targets
            .into_iter()
            .map(|target| target.deps)
            .flatten()
            .for_each(|dep_id| {
                util::add_leaf_node(&mut graph, &mut id_to_ix_map, dep_id);
            });

        // add edges
        graph
            .graph()
            .node_indices()
            .map(|target_ix| {
                util::add_edges_to_deps(&mut graph, &id_to_ix_map, target_ix)
            })
            .collect::<Result<_>>()?;

        Ok(Self {
            graph,
            id_to_ix_map,
        })
    }

    pub fn get_target_sequence(
        &self,
        target_id: Identifier,
    ) -> Result<Vec<Target>> {
        let graph = &self.graph;
        let target_ix = *self
            .id_to_ix_map
            .get(&target_id)
            .ok_or_else(|| UserError::NoSuchTarget(target_id))?;

        let depth_map = util::generate_depth_map(graph, target_ix);
        let obsolete_leaf_nodes =
            util::find_obsolete_leaf_nodes(graph.graph())?;
        let obsolete_targets =
            util::find_obsolete_targets(graph.graph(), &obsolete_leaf_nodes);

        util::get_target_sequence(graph.graph(), &depth_map, &obsolete_targets)
    }
}

mod util {
    use std::collections::{HashMap, VecDeque};

    use super::*;

    use daggy::petgraph;
    use petgraph::prelude::{Direction, Graph};

    pub(super) fn get_target_sequence(
        graph: &Graph<Node, ()>,
        depth_map: &HashMap<Nx, usize>,
        obsolete_targets: &HashSet<Nx>,
    ) -> Result<Vec<Target>> {
        // filter out targets which are not in the
        // dependency graph of the chosen target
        // and sort the targets left by depth in **decreasing** order
        let mut obsolete_targets = obsolete_targets
            .iter()
            .filter(|ix| depth_map.contains_key(ix))
            .copied()
            .collect::<Vec<_>>();
        obsolete_targets.sort_by_key(|ix| depth_map[ix]);
        obsolete_targets.reverse();

        obsolete_targets
            .into_iter()
            .map(|target_ix| match &graph[target_ix] {
                Node::Target(target) => Ok(target.clone()),
                Node::NoRule(_) => Err(Error::internal(line!(), file!())),
            })
            .collect::<Result<Vec<_>>>()
    }

    /// This function finds all nodes that have no dependencies -
    /// both actual files (`NoRule` variant) and targets
    /// (`Target` variant with no dependencies is assumed to depend
    /// on other factors - time, environmental variables,
    /// current directory etc.).
    pub(super) fn find_obsolete_leaf_nodes(
        graph: &Graph<Node, ()>,
    ) -> Result<HashSet<Nx>> {
        graph
            .externals(Direction::Outgoing) // get nodes with no outgoing edges
            .filter_map(|node_ix| match &graph[node_ix] {
                // TODO filter `requires_rebuilding`
                Node::Target(_target) => Some(Ok(node_ix)),
                Node::NoRule(identifier) => {
                    // TODO clean up this part
                    let previous_file_state = ();
                    let result = has_file_been_modified(
                        &identifier,
                        previous_file_state,
                    );

                    match result {
                        Ok(has_been_modified) =>
                            if has_been_modified {
                                Some(Ok(node_ix))
                            } else {
                                None
                            },
                        Err(err) => Some(Err(err)),
                    }
                }
            })
            .collect::<Result<HashSet<_>>>()
    }

    pub(super) fn find_obsolete_targets(
        graph: &Graph<Node, ()>,
        obsolete_leaf_nodes: &HashSet<Nx>,
    ) -> HashSet<Nx> {
        // reverse short circuiting bfs:
        // skip the dependants of the targets
        // that have already been marked as obsolete
        let mut queue = VecDeque::<Nx>::new();
        let mut obsolete_ixs = HashSet::<Nx>::new();

        for leaf_ix in obsolete_leaf_nodes {
            // no need to clear the queue since it gets drained
            // in the while loop each time

            match &graph[*leaf_ix] {
                Node::Target(_) => queue.push_back(*leaf_ix),
                Node::NoRule(_) => {
                    let direct_dependants =
                        graph.neighbors_directed(*leaf_ix, Direction::Incoming);
                    queue.extend(direct_dependants);
                }
            }

            while let Some(target_ix) = queue.pop_front() {
                let has_just_been_found = obsolete_ixs.insert(target_ix);

                if has_just_been_found {
                    let dependants = graph
                        .neighbors_directed(target_ix, Direction::Incoming);
                    queue.extend(dependants);
                }
            }
        }

        obsolete_ixs
    }

    pub(super) fn add_leaf_node(
        graph: &mut DependencyDag,
        id_to_ix_map: &mut HashMap<Identifier, Nx>,
        dependency_identifier: Identifier,
    ) {
        id_to_ix_map
            .entry(dependency_identifier.clone())
            .or_insert_with(|| {
                // `.add_node()` returns node's index
                graph.add_node(Node::NoRule(dependency_identifier))
            });
    }

    pub(super) fn add_target_node(
        graph: &mut DependencyDag,
        id_to_ix_map: &mut HashMap<Identifier, Nx>,
        target: Target,
    ) -> Result<()> {
        let identifier = target.identifier.clone();
        let node_ix = graph.add_node(Node::Target(target));
        let slot = id_to_ix_map.insert(identifier, node_ix);

        match slot {
            Some(_colliding_target_ix) =>
                Err(UserError::DuplicateTarget.into()),
            None => Ok(()),
        }
    }

    pub(super) fn add_edges_to_deps(
        graph: &mut DependencyDag,
        id_to_ix_map: &HashMap<Identifier, Nx>,
        target_ix: Nx,
    ) -> Result<()> {
        let deps = match &graph[target_ix] {
            Node::Target(target) => target.deps.clone(),
            Node::NoRule(_) => return Ok(()), // no deps
        };

        deps.iter()
            .map(|dep_id| {
                id_to_ix_map
                    .get(dep_id)
                    .ok_or_else(|| Error::internal(line!(), file!()))
            })
            .map_item(|dep_ix| {
                graph
                    .add_edge(target_ix, *dep_ix, ())
                    .map(|_| ())
                    .map_err(|_| UserError::DependencyCycle.into())
            })
            .map(|result| result.flatten())
            .collect::<Result<_>>()
    }

    pub(super) fn has_file_been_modified(
        _identifier: &Identifier,
        _previous_state: FileState,
    ) -> Result<bool> {
        Ok(true) // TODO for now it just rebuilds everything
    }

    pub(super) fn generate_depth_map<N, E>(
        graph: &daggy::Dag<N, E>,
        target_id: Nx,
    ) -> HashMap<Nx, usize> {
        let mut depth_map: HashMap<Nx, usize> = HashMap::new();
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

                let children =
                    graph.neighbors_directed(current_node, Direction::Outgoing);
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
    use pretty_assertions::assert_eq;

    #[test]
    fn test_get_task_sequence() {
        // helper functions
        let task = |cmd: &str| Task {
            command: cmd.into(),
            working_dir: None,
        };
        let target = |id: &str, deps: &[&str]| Target {
            identifier: id.into(),
            deps: deps.iter().map(|d| d.into()).collect(),
            tasks: vec![task(id)],
            working_dir: None,
        };
        let ix = |id: &str, map: &HashMap<_, _>| {
            let p: &std::path::Path = id.as_ref();
            map[p]
        };

        // the dependency graph:
        //
        //     a1      a2'
        //    /       /  \
        //   /       /    \
        // b1      b2      b3
        //        /       /
        //       /       /
        //     l1*     l2
        //
        // a2 is the target (')
        // l1 is marked as obsolete (*)
        // b2's and a2's tasks must be executed (in that order)

        // targets and their dependencies
        #[rustfmt::skip]
        let targets = vec![
            target("a1", &["b1"]),
            target("a2", &["b2", "b3"]),
            target("b2", &["l1"]),
            target("b3", &["l2"]),
        ];
        let DependencyGraph {
            graph,
            id_to_ix_map: map,
        } = DependencyGraph::construct(targets).unwrap();

        // depth map
        #[rustfmt::skip]
        let depth_map = vec![
            (ix("a2", &map), 0),
            (ix("b2", &map), 1),
            (ix("b3", &map), 1),
            (ix("l1", &map), 2),
            (ix("l2", &map), 2),
        ].into_iter().collect();

        // nodes that have been marked as obsolete
        // (in real code it is automated)
        let obsolete_leaf_nodes = vec![ix("l1", &map)].into_iter().collect();

        // get the sequence of tasks that must be executed
        // in specific order
        let obsolete_targets =
            util::find_obsolete_targets(graph.graph(), &obsolete_leaf_nodes);
        let target_sequence = util::get_target_sequence(
            graph.graph(),
            &depth_map,
            &obsolete_targets,
        )
        .unwrap();
        let target_sequence = target_sequence
            .into_iter()
            .map(|target| target.identifier)
            .collect::<Vec<_>>();
        let expected_target_sequence: Vec<PathBuf> =
            vec!["b2".into(), "a2".into()];

        assert_eq!(target_sequence, expected_target_sequence);
    }

    #[test]
    fn test_find_obsolete_targets() {
        // helper functions
        let target = |id: &str, deps: &[&str]| Target {
            identifier: id.into(),
            deps: deps.iter().map(|d| d.into()).collect(),
            tasks: vec![],
            working_dir: None,
        };
        let ixs = |ids: &[&str], map: &HashMap<_, Nx>| {
            ids.iter()
                .map(|id| map[&Into::<PathBuf>::into(id)])
                .collect()
        };

        // the dependency graph:
        //
        //     a1      a2
        //    /  \       \
        //   /    \       \
        // b1      b2      b3
        //        /       /
        //       /       /
        //     l1*     l2*
        //
        // l1 and l2 are marked as obsolete
        // the function should find b2, a1, b3 & a2
        // but not b1

        #[rustfmt::skip]
        let targets = vec![
            target("a1", &["b1", "b2"]),
            target("a2", &["b3"]),
            target("b2", &["l1"]),
            target("b3", &["l2"]),
        ];
        let DependencyGraph {
            graph,
            id_to_ix_map: map,
        } = DependencyGraph::construct(targets).unwrap();
        let obsolete_leaf_nodes = ixs(&["l1", "l2"], &map);

        let found_targets =
            util::find_obsolete_targets(&graph.graph(), &obsolete_leaf_nodes);
        let expected_targets = ixs(&["a1", "a2", "b2", "b3"], &map);

        assert_eq!(found_targets, expected_targets);
    }

    #[test]
    fn test_generate_depth_map() {
        // depth is the length of the longest path from
        // the target node to the dependency
        #[rustfmt::skip]
        let graph: daggy::Dag<(), ()> = daggy::Dag::from_edges(&[
            (0, 3), (0, 4),
            (1, 3), (1, 4), (1, 6),
            (2, 3), (2, 4),
            (3, 5), (3, 6), (3, 7),
            (4, 5), (4, 6), (4, 7),
            (7, 8),
            (8, 9),
        ]).unwrap();

        let target = n(1); // target
        let depth_map = util::generate_depth_map(&graph, target);

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
