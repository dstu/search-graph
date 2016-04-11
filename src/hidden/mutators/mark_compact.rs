//! Mark-and-compact garbage collection for pruning graphs.
//!
//! This module implements a mark-and-compact garbage collector that can prune a
//! graph so that only components reachable from a set of root game states are
//! retained. Running time and memory required are linear in graph size,
//! although there is a potential for a high cost when rebuilding the hashtable
//! that maps from game states to their IDs.

use ::{Graph, Target};
use ::hidden::base::{ArcId, StateId};

use std::cmp::Eq;
use std::collections::VecDeque;
use std::hash::Hash;
use std::mem;

/// Permutes `data` so that element `i` of data is reassigned to be at index
/// `f(id_map[i])`.
///
/// Elements `j` of `data` for which `id_map[j]` is `None` are discarded.
fn permute_compact<T, F>(data: &mut Vec<T>, f: F) where F: Fn(usize) -> Option<usize> {
    if data.is_empty() {
        return
    }

    // TODO: We should benchmark doing this in-place vs. via moving.
    let mut new_data: Vec<T> = Vec::with_capacity(data.len());
    let mut retained_count = 0;
    {
        let compacted = data.drain(0..).enumerate()
            .filter_map(|(old_index, t)| f(old_index).map(|new_index| (new_index, t)));
        for (new_index, mut t) in compacted {
            mem::swap(unsafe { new_data.get_unchecked_mut(new_index) }, &mut t);
            mem::forget(t);
            retained_count += 1;
        }
    }
    unsafe { new_data.set_len(retained_count) };  // Maybe do this after each swap?
    mem::replace(data, new_data);
}

/// Garbage collector state.
pub struct Collector<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    graph: &'a mut Graph<T, S, A>,
    marked_state_count: usize,
    marked_arc_count: usize,
    state_id_map: Vec<Option<StateId>>,
    arc_id_map: Vec<Option<ArcId>>,
    frontier: VecDeque<StateId>,
}

impl<'a, T, S, A> Collector<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    /// Runs mark-and-sweep garbage collection on a graph. Graph components not
    /// reachable from the vertices corresponding to roots will be dropped.
    ///
    /// This is intended as the main entrypoint for this module. But this
    /// function is not exported by the crate, so you probably want the
    /// `retain_reachable()` method of `MutNode` or the `retain_reachable_from`
    /// method of `Graph`.
    pub fn retain_reachable(graph: &'a mut Graph<T, S, A>, roots: &[StateId]) {
        let mut c = Collector::new(graph);
        c.mark(roots);
        c.sweep();
    }

    /// Creates a new mark-and-sweep garbage collector with empty initial state.
    fn new(graph: &'a mut Graph<T, S, A>) -> Self {
        let empty_states = vec!(None; graph.vertices.len());
        let empty_arcs = vec!(None; graph.arcs.len());
        Collector {
            graph: graph,
            marked_state_count: 0,
            marked_arc_count: 0,
            state_id_map: empty_states,
            arc_id_map: empty_arcs,
            frontier: VecDeque::new(),
        }
    }

    /// Traverses graph components reachable from `roots` and marks them as
    /// reachable. Also builds a new graph component addressing scheme that
    /// reassigns `StateId` and `ArcId` values.
    ///
    /// As side effects, arc sources and vertex children are updated to use the
    /// new addressing scheme.
    fn mark(&mut self, roots: &[StateId]) {
        for id in roots.iter() {
            Self::remap_state_id(&mut self.state_id_map, &mut self.marked_state_count, *id);
            self.frontier.push_back(*id);
        }
        while self.mark_next() { }
    }

    /// Looks up the mapping between old and new StateIds. May update
    /// `state_id_map` with a new mapping, given that we have remapped
    /// `marked_state_count` StateIds so far.
    fn remap_state_id(state_id_map: &mut [Option<StateId>], marked_state_count: &mut usize,
                      old_state_id: StateId) -> StateId {
        let index = old_state_id.as_usize();
        if let Some(new_state_id) = state_id_map[index] {
            return new_state_id
        }
        let new_state_id = StateId(*marked_state_count);
        state_id_map[index] = Some(new_state_id);
        *marked_state_count += 1;
        new_state_id
    }
    
    /// Looks up the mapping between old and new ArcIds. May update
    /// `arc_id_map` with a new mapping, given that we have remapped
    /// `marked_arc_count` ArcIds so far.
    fn remap_arc_id(arc_id_map: &mut [Option<ArcId>], marked_arc_count: &mut usize,
                    old_arc_id: ArcId) -> ArcId {
        let index = old_arc_id.as_usize();
        if let Some(new_arc_id) = arc_id_map[index] {
            return new_arc_id
        }
        let new_arc_id = ArcId(*marked_arc_count);
        arc_id_map[index] = Some(new_arc_id);
        *marked_arc_count += 1;
        new_arc_id
    }

    fn mark_next(&mut self) -> bool{
        match self.frontier.pop_front() {
            None => false,
            Some(state_id) => {
                let (new_state_id, mut child_arc_ids): (StateId, Vec<ArcId>) = {
                    let vertex = self.graph.get_vertex_mut(state_id);
                    (self.state_id_map[state_id.as_usize()].unwrap(),
                     vertex.children.drain(0..).collect())
                };

                for arc_id in child_arc_ids.iter_mut() {
                    let arc = self.graph.get_arc_mut(*arc_id);
                    // Update arc sources to use new state IDs.
                    arc.source = new_state_id;
                    if let Target::Expanded(child_vertex_id) = arc.target {
                        if self.state_id_map[child_vertex_id.as_usize()].is_none() {
                            Self::remap_state_id(
                                &mut self.state_id_map, &mut self.marked_state_count, child_vertex_id);
                            self.frontier.push_back(child_vertex_id);
                        }
                    }
                    let new_arc_id =
                        Self::remap_arc_id(&mut self.arc_id_map, &mut self.marked_arc_count, *arc_id);
                    self.arc_id_map[arc_id.as_usize()] = Some(new_arc_id);
                    *arc_id = new_arc_id;
                }

                // Update vertex children to use new ArcIds.
                self.graph.get_vertex_mut(state_id).children = child_arc_ids;
                true
            },
        }
    }

    /// Drops vertices which were not reached in the previous `mark()`. Must be
    /// run after `mark()`.
    ///
    /// Also, updates vertex pointers to parent edges to use the new `ArcId`
    /// addressing scheme built in the previous call to `mark()`.
    fn sweep(&mut self) {
        let state_id_map = {
            let mut state_id_map = Vec::new();
            mem::swap(&mut state_id_map, &mut self.state_id_map);
            state_id_map
        };
        let arc_id_map = {
            let mut arc_id_map = Vec::new();
            mem::swap(&mut arc_id_map, &mut self.arc_id_map);
            arc_id_map
        };
        // Compact marked vertices.
        permute_compact(&mut self.graph.vertices, |i| state_id_map[i].map(|id| id.as_usize()));
        // Drop unmarked vertices.
        self.graph.vertices.truncate(self.marked_state_count);
        // Reassign and compact vertex parents.
        for mut vertex in self.graph.vertices.iter_mut() {
            let mut store_index = 0;
            for scan_index in 0..vertex.parents.len() {
                let old_arc_id = vertex.parents[scan_index];
                if let Some(new_arc_id) = arc_id_map[old_arc_id.as_usize()] {
                    vertex.parents[store_index] = new_arc_id;
                    store_index += 1;
                }
            }
            vertex.parents.truncate(store_index);
            vertex.parents.shrink_to_fit();
        }

        // Compact marked arcs.
        permute_compact(&mut self.graph.arcs, |i| arc_id_map[i].map(|id| id.as_usize()));
        // Reassign arc targets.
        for mut arc in self.graph.arcs.iter_mut() {
            arc.target = match arc.target {
                Target::Expanded(old_state_id) =>
                    Target::Expanded(state_id_map[old_state_id.as_usize()].unwrap()),
                x @ _ => x,
            };
        }

        // Update state namespace to use new mapping.
        self.graph.state_ids.remap(|_, old_state_id| state_id_map[old_state_id.as_usize()]);
    }
}

#[cfg(test)]
mod test {
    use super::Collector;
    use ::hidden::base::{ArcId, Arc, StateId, Vertex};
    use ::Target;

    type Graph = ::Graph<&'static str, &'static str, &'static str>;

    fn empty_graph() -> Graph {
        let g = Graph::new();
        assert_eq!(0, g.vertex_count());
        assert_eq!(0, g.edge_count());
        g
    }

    fn make_vertex(data: &'static str, parents: Vec<ArcId>, children: Vec<ArcId>)
                   -> Vertex<&'static str> {
        Vertex { data: data, parents: parents, children: children, }
    }

    fn make_arc(data: &'static str, source: StateId, target: Target<StateId, ()>)
                -> Arc<&'static str> {
        Arc { data: data, source: source, target: target, }
    }

    #[test]
    fn empty_graph_ok() {
        let mut g = empty_graph();
        Collector::retain_reachable(&mut g, &[]);
        assert_eq!(0, g.vertex_count());
        assert_eq!(0, g.edge_count());
    }

    #[test]
    fn mark_roots_ok() {
        let mut g = empty_graph();
        g.add_root("0", "");
        g.add_root("1", "");
        g.add_root("2", "");
        assert_eq!(3, g.vertex_count());
        assert_eq!(0, g.edge_count());
        let root_ids = [StateId(0), StateId(1), StateId(2)];
        let mut c = Collector::new(&mut g);
        c.mark(&root_ids);
        for (i, new_id) in c.state_id_map.iter().enumerate() {
            if new_id.is_some() {
                assert!(root_ids.contains(&StateId(i)));
            }
        }
    }

    #[test]
    fn reachable_loop_ok() {
        let mut g = empty_graph();
        // Original StateIds are:
        // "0": 0
        // "00": 1
        // "01": 2
        // "1": 3
        // "10": 4
        // "11": 5
        // "2": 6
        // "20": 7
        // "21": 8
        // "210": 9
        // "211": 10
        // "2100": 11

        // Original ArcIds are:
        // "0" -> "00": 0
        // "0" -> "01": 1
        // "1" -> "10": 2
        // "1" -> "11": 3
        // "11" -> "0": 4
        // "2" -> "20": 5
        // "2" -> "21": 6
        // "21" -> "210": 7
        // "21" -> "211": 8
        // "210" -> "0": 9
        // "210" -> "2100": 10
        // "2100" -> "0": 11

        g.add_edge("0", |_| "0_data",
                   "00", |_| "00_data",
                   "0_00_data");
        g.add_edge("0", |_| "0_data",
                   "01", |_| "01_data",
                   "0_01_data");
        g.add_edge("1", |_| "1_data",
                   "10", |_| "10_data",
                   "1_10_data");
        g.add_edge("1", |_| "1_data",
                   "11", |_| "11_data",
                   "1_11_data");
        g.add_edge("11", |_| "11_data",
                   "0", |_| "0_data",
                   "11_0_data");
        g.add_edge("2", |_| "2_data",
                   "20", |_| "20_data",
                   "2_20_data");
        g.add_edge("2", |_| "2_data",
                   "21", |_| "21_data",
                   "2_21_data");
        g.add_edge("21", |_| "21_data",
                   "210", |_| "210_data",
                   "21_210_data");
        g.add_edge("21", |_| "21_data",
                   "211", |_| "211_data",
                   "21_211_data");
        g.add_edge("210", |_| "210_data",
                   "0", |_| "0_data",
                   "210_0_data");
        g.add_edge("210", |_| "210_data",
                   "2100", |_| "2100_data",
                   "210_2100_data");
        g.add_edge("2100", |_| "2100_data",
                   "0", |_| "0_data",
                   "2100_0_data");

        let root_ids = [StateId(6)];
        let reachable_state_ids = [
            StateId(6), StateId(7), StateId(8), StateId(9), StateId(10), StateId(11),
            StateId(0), StateId(1), StateId(2)];
        let unreachable_state_ids = [StateId(3), StateId(4), StateId(5)];

        // Mark.
        let mut c = Collector::new(&mut g);
        c.mark(&root_ids);

        for (i, new_id) in c.state_id_map.iter().enumerate() {
            if new_id.is_some() {
                // Reachable IDs are remapped.
                assert!(reachable_state_ids.contains(&StateId(i)));
            } else {
                // Unreachable IDs aren't.
                assert!(unreachable_state_ids.contains(&StateId(i)));
            }
        }

        // New StateIds are:
        // "2": 0
        // "20": 1
        // "21": 2
        // "210": 3
        // "211": 4
        // "0": 5
        // "2100": 6
        // "00": 7
        // "01": 8
        // This is BFS order, as we eagerly remap StateIds when we first
        // encounter them, not when they are visited. This should help by
        // compacting memory so that child vertices are adjacent to one another.
        assert_eq!(c.state_id_map,
                   vec!(Some(StateId(5)),
                        Some(StateId(7)),
                        Some(StateId(8)),
                        None,
                        None,
                        None,
                        Some(StateId(0)),
                        Some(StateId(1)),
                        Some(StateId(2)),
                        Some(StateId(3)),
                        Some(StateId(4)),
                        Some(StateId(6)),));

        // New ArcIds are:
        // "2" -> "20": 0
        // "2" -> "21": 1
        // "21" -> "210": 2
        // "21" -> "211": 3
        // "210" -> "0": 4
        // "210" -> "2100": 5
        // "0" -> "00": 6
        // "0" -> "01": 7
        // "2100" -> "0": 8
        // Again, this places child arc data in contiguous segments of memory.
        assert_eq!(c.arc_id_map,
                   vec!(Some(ArcId(6)),
                        Some(ArcId(7)),
                        None,
                        None,
                        None,
                        Some(ArcId(0)),
                        Some(ArcId(1)),
                        Some(ArcId(2)),
                        Some(ArcId(3)),
                        Some(ArcId(4)),
                        Some(ArcId(5)),
                        Some(ArcId(8)),));

        c.sweep();
        assert_eq!(c.graph.vertices,
                   vec!(make_vertex("2_data",
                                    vec![],
                                    vec![ArcId(0), ArcId(1)],),
                        make_vertex("20_data",
                                    vec![ArcId(0)],
                                    vec![]),
                        make_vertex("21_data",
                                    vec![ArcId(1)],
                                    vec![ArcId(2), ArcId(3)]),
                        make_vertex("210_data",
                                    vec![ArcId(2)],
                                    vec![ArcId(4), ArcId(5)]),
                        make_vertex("211_data",
                                    vec![ArcId(3)],
                                    vec![]),
                        make_vertex("0_data",
                                    vec![ArcId(4), ArcId(8)],
                                    vec![ArcId(6), ArcId(7)]),
                        make_vertex("2100_data",
                                    vec![ArcId(5)],
                                    vec![ArcId(8)]),
                        make_vertex("00_data",
                                    vec![ArcId(6)],
                                    vec![]),
                        make_vertex("01_data",
                                    vec![ArcId(7)],
                                    vec![]),));

        assert_eq!(c.graph.arcs,
                   vec!(make_arc("2_20_data", StateId(0), Target::Expanded(StateId(1))),
                        make_arc("2_21_data", StateId(0), Target::Expanded(StateId(2))),
                        make_arc("21_210_data", StateId(2), Target::Expanded(StateId(3))),
                        make_arc("21_211_data", StateId(2), Target::Expanded(StateId(4))),
                        make_arc("210_0_data", StateId(3), Target::Expanded(StateId(5))),
                        make_arc("210_2100_data", StateId(3), Target::Expanded(StateId(6))),
                        make_arc("0_00_data", StateId(5), Target::Expanded(StateId(7))),
                        make_arc("0_01_data", StateId(5), Target::Expanded(StateId(8))),
                        make_arc("2100_0_data", StateId(6), Target::Expanded(StateId(5))),));

        // TODO: Tests of state namespace.
    }

    // TODO: Test that unexpanded arcs are handled correctly.

    // TODO: Test that parallel edges are handled correctly.
}
