//! Mark-and-compact garbage collection for pruning graphs.
//!
//! This module implements a mark-and-compact garbage collector that can prune a
//! graph so that only components reachable from a set of root game states are
//! retained. Running time and memory required are linear in graph size,
//! although there is a potential for a high cost when rebuilding the hashtable
//! that maps from game states to their IDs.

use std::cmp::Eq;
use std::collections::VecDeque;
use std::hash::Hash;
use std::mem;
use std::ptr;

use ::Graph;
use ::hidden::base::{EdgeId, VertexId};
use ::symbol_table::SymbolId;
use ::symbol_table::indexing::{HashIndexing, Indexing};

/// Permutes `data` so that element `i` of data is reassigned to be at index
/// `f(i)`.
///
/// Elements `j` of `data` for which `f(j)` is `None` are discarded.
fn permute_compact<T, F>(data: &mut Vec<T>, f: F) where F: Fn(usize) -> Option<usize> {
    if data.is_empty() {
        return
    }

    // TODO: We should benchmark doing this in-place vs. via moving.
    let mut new_data: Vec<T> = Vec::with_capacity(data.len());
    // TODO: This relies on an implementation detail of Vec (namely, that
    // Vec::with_capacity gives us a block that we can read into with
    // get_unchecked_mut, even if the index we're accessing is beyond the length
    // of the Vec). This seems unlikely to change, but it may ultimately be more
    // future-proof to allocate a block of memory, do writes into it manually,
    // and pass it to Vec::from_raw_parts.
    let mut retained_count = 0;
    {
        let compacted = data.drain(..).enumerate()
            .filter_map(|(old_index, t)| f(old_index).map(|new_index| (new_index, t)));
        for (new_index, t) in compacted {
            unsafe { ptr::write(new_data.get_unchecked_mut(new_index), t) };
            retained_count += 1;
        }
    }
    unsafe { new_data.set_len(retained_count) };  // TODO: Maybe do this after each swap?
    mem::replace(data, new_data);
}

/// Garbage collector state.
pub struct Collector<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    graph: &'a mut Graph<T, S, A>,
    marked_state_count: usize,
    marked_arc_count: usize,
    state_id_map: Vec<Option<VertexId>>,
    arc_id_map: Vec<Option<EdgeId>>,
    frontier: VecDeque<VertexId>,
}

impl<'a, T, S, A> Collector<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    /// Runs mark-and-sweep garbage collection on a graph. Graph components not
    /// reachable from the vertices corresponding to roots will be dropped.
    ///
    /// This is intended as the main entrypoint for this module. But this
    /// function is not exported by the crate, so you probably want the
    /// `retain_reachable()` method of `MutNode` or the `retain_reachable_from`
    /// method of `Graph`.
    pub fn retain_reachable(graph: &'a mut Graph<T, S, A>, roots: &[VertexId]) {
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
    /// reassigns `VertexId` and `EdgeId` values.
    ///
    /// As side effects, arc sources and vertex children are updated to use the
    /// new addressing scheme.
    fn mark(&mut self, roots: &[VertexId]) {
        for id in roots.iter() {
            Self::remap_state_id(&mut self.state_id_map, &mut self.marked_state_count, *id);
            self.frontier.push_back(*id);
        }
        while self.mark_next() { }
    }

    /// Looks up the mapping between old and new VertexIds. May update
    /// `state_id_map` with a new mapping, given that we have remapped
    /// `marked_state_count` VertexIds so far.
    fn remap_state_id(state_id_map: &mut [Option<VertexId>], marked_state_count: &mut usize,
                      old_state_id: VertexId) -> VertexId {
        let index = old_state_id.as_usize();
        if let Some(new_state_id) = state_id_map[index] {
            return new_state_id
        }
        let new_state_id = VertexId(*marked_state_count);
        state_id_map[index] = Some(new_state_id);
        *marked_state_count += 1;
        new_state_id
    }
    
    /// Looks up the mapping between old and new EdgeIds. May update
    /// `arc_id_map` with a new mapping, given that we have remapped
    /// `marked_arc_count` EdgeIds so far.
    fn remap_arc_id(arc_id_map: &mut [Option<EdgeId>], marked_arc_count: &mut usize,
                    old_arc_id: EdgeId) -> EdgeId {
        let index = old_arc_id.as_usize();
        if let Some(new_arc_id) = arc_id_map[index] {
            return new_arc_id
        }
        let new_arc_id = EdgeId(*marked_arc_count);
        arc_id_map[index] = Some(new_arc_id);
        *marked_arc_count += 1;
        new_arc_id
    }

    fn mark_next(&mut self) -> bool{
        match self.frontier.pop_front() {
            None => false,
            Some(state_id) => {
                let (new_state_id, mut child_arc_ids): (VertexId, Vec<EdgeId>) = {
                    let vertex = self.graph.get_vertex_mut(state_id);
                    (self.state_id_map[state_id.as_usize()].unwrap(),
                     vertex.children.drain(0..).collect())
                };

                for arc_id in child_arc_ids.iter_mut() {
                    let arc = self.graph.get_arc_mut(*arc_id);
                    // Update arc sources to use new state IDs.
                    arc.source = new_state_id;
                    if self.state_id_map[arc.target.as_usize()].is_none() {
                        Self::remap_state_id(
                            &mut self.state_id_map, &mut self.marked_state_count, arc.target);
                        self.frontier.push_back(arc.target);
                    }
                    let new_arc_id =
                        Self::remap_arc_id(&mut self.arc_id_map, &mut self.marked_arc_count, *arc_id);
                    self.arc_id_map[arc_id.as_usize()] = Some(new_arc_id);
                    *arc_id = new_arc_id;
                }

                // Update vertex children to use new EdgeIds.
                self.graph.get_vertex_mut(state_id).children = child_arc_ids;
                true
            },
        }
    }

    /// Drops vertices which were not reached in the previous `mark()`. Must be
    /// run after `mark()`.
    ///
    /// Also, updates vertex pointers to parent edges to use the new `EdgeId`
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
            arc.target = state_id_map[arc.target.as_usize()].unwrap();
        }

        // Update state namespace to use new mapping.
        let mut new_state_ids = HashIndexing::default();
        mem::swap(&mut new_state_ids, &mut self.graph.state_ids);
        let mut table = new_state_ids.to_table();
        table.remap(|symbol| state_id_map[symbol.id().as_usize()]);
        self.graph.state_ids = HashIndexing::from_table(table);
    }
}

#[cfg(test)]
mod test {
    use super::Collector;
    use ::hidden::base::{EdgeId, VertexId, RawEdge, RawVertex};
    use ::symbol_table::indexing::{HashIndexing, Indexing};

    use std::collections::HashMap;
    use std::mem;

    type Graph = ::Graph<&'static str, &'static str, &'static str>;

    fn empty_graph() -> Graph {
        let g = Graph::new();
        assert_eq!(0, g.vertex_count());
        assert_eq!(0, g.edge_count());
        g
    }

    fn make_vertex(data: &'static str, parents: Vec<EdgeId>, children: Vec<EdgeId>)
                   -> RawVertex<&'static str> {
        RawVertex { data: data, parents: parents, children: children, }
    }

    fn make_arc(data: &'static str, source: VertexId, target: VertexId)
                -> RawEdge<&'static str> {
        RawEdge { data: data, source: source, target: target, }
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
        let root_ids = [VertexId(0), VertexId(1), VertexId(2)];
        let mut c = Collector::new(&mut g);
        c.mark(&root_ids);
        for (i, new_id) in c.state_id_map.iter().enumerate() {
            if new_id.is_some() {
                assert!(root_ids.contains(&VertexId(i)));
            }
        }
    }

    #[test]
    fn reachable_loop_ok() {
        let mut g = empty_graph();
        // Original VertexIds are:
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

        // Original EdgeIds are:
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

        let root_ids = [VertexId(6)];
        let reachable_state_ids = [
            VertexId(6), VertexId(7), VertexId(8), VertexId(9), VertexId(10), VertexId(11),
            VertexId(0), VertexId(1), VertexId(2)];
        let unreachable_state_ids = [VertexId(3), VertexId(4), VertexId(5)];

        // Mark.
        let mut c = Collector::new(&mut g);
        c.mark(&root_ids);

        for (i, new_id) in c.state_id_map.iter().enumerate() {
            if new_id.is_some() {
                // Reachable IDs are remapped.
                assert!(reachable_state_ids.contains(&VertexId(i)));
            } else {
                // Unreachable IDs aren't.
                assert!(unreachable_state_ids.contains(&VertexId(i)));
            }
        }

        // New VertexIds are:
        // "2": 0
        // "20": 1
        // "21": 2
        // "210": 3
        // "211": 4
        // "0": 5
        // "2100": 6
        // "00": 7
        // "01": 8
        // This is BFS order, as we eagerly remap VertexIds when we first
        // encounter them, not when they are visited. This should help by
        // compacting memory so that child vertices are adjacent to one another.
        assert_eq!(c.state_id_map,
                   vec!(Some(VertexId(5)),
                        Some(VertexId(7)),
                        Some(VertexId(8)),
                        None,
                        None,
                        None,
                        Some(VertexId(0)),
                        Some(VertexId(1)),
                        Some(VertexId(2)),
                        Some(VertexId(3)),
                        Some(VertexId(4)),
                        Some(VertexId(6)),));

        // New EdgeIds are:
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
                   vec!(Some(EdgeId(6)),
                        Some(EdgeId(7)),
                        None,
                        None,
                        None,
                        Some(EdgeId(0)),
                        Some(EdgeId(1)),
                        Some(EdgeId(2)),
                        Some(EdgeId(3)),
                        Some(EdgeId(4)),
                        Some(EdgeId(5)),
                        Some(EdgeId(8)),));

        c.sweep();
        assert_eq!(c.graph.vertices,
                   vec!(make_vertex("2_data",
                                    vec!(),
                                    vec!(EdgeId(0), EdgeId(1)),),
                        make_vertex("20_data",
                                    vec!(EdgeId(0)),
                                    vec!()),
                        make_vertex("21_data",
                                    vec!(EdgeId(1)),
                                    vec!(EdgeId(2), EdgeId(3))),
                        make_vertex("210_data",
                                    vec!(EdgeId(2)),
                                    vec!(EdgeId(4), EdgeId(5))),
                        make_vertex("211_data",
                                    vec!(EdgeId(3)),
                                    vec!()),
                        make_vertex("0_data",
                                    vec!(EdgeId(4), EdgeId(8)),
                                    vec!(EdgeId(6), EdgeId(7))),
                        make_vertex("2100_data",
                                    vec!(EdgeId(5)),
                                    vec!(EdgeId(8))),
                        make_vertex("00_data",
                                    vec!(EdgeId(6)),
                                    vec!()),
                        make_vertex("01_data",
                                    vec!(EdgeId(7)),
                                    vec!()),));

        assert_eq!(c.graph.arcs,
                   vec!(make_arc("2_20_data", VertexId(0), VertexId(1)),
                        make_arc("2_21_data", VertexId(0), VertexId(2)),
                        make_arc("21_210_data", VertexId(2), VertexId(3)),
                        make_arc("21_211_data", VertexId(2), VertexId(4)),
                        make_arc("210_0_data", VertexId(3), VertexId(5)),
                        make_arc("210_2100_data", VertexId(3), VertexId(6)),
                        make_arc("0_00_data", VertexId(5), VertexId(7)),
                        make_arc("0_01_data", VertexId(5), VertexId(8)),
                        make_arc("2100_0_data", VertexId(6), VertexId(5)),));

        let mut state_associations = HashMap::new();
        state_associations.insert("2", VertexId(0));
        state_associations.insert("20", VertexId(1));
        state_associations.insert("21", VertexId(2));
        state_associations.insert("210", VertexId(3));
        state_associations.insert("211", VertexId(4));
        state_associations.insert("0", VertexId(5));
        state_associations.insert("2100", VertexId(6));
        state_associations.insert("00", VertexId(7));
        state_associations.insert("01", VertexId(8));
        let mut state_ids = HashIndexing::default();
        mem::swap(&mut state_ids, &mut c.graph.state_ids);
        assert_eq!(state_ids.to_table().to_hash_map(), state_associations);
    }

    #[test]
    fn parallel_edges_ok() {
        let mut g = empty_graph();
        g.add_edge("0", |_| "0_data", "00", |_| "00_data", "0_00_data_1");
        g.add_edge("0", |_| "0_data", "00", |_| "00_data", "0_00_data_2");
        g.add_edge("0", |_| "0_data", "01", |_| "01_data", "0_01_data");
        g.add_edge("1", |_| "1_data", "10", |_| "10_data", "1_10_data");
        g.add_edge("1", |_| "1_data", "1", |_| "1_data", "1_1_data");
        assert_eq!(g.vertices,
                   vec!(make_vertex("0_data", vec!(), vec!(EdgeId(0), EdgeId(1), EdgeId(2))),
                        make_vertex("00_data", vec!(EdgeId(0), EdgeId(1)), vec!()),
                        make_vertex("01_data", vec!(EdgeId(2)), vec!()),
                        make_vertex("1_data", vec!(EdgeId(4)), vec!(EdgeId(3), EdgeId(4))),
                        make_vertex("10_data", vec!(EdgeId(3)), vec!())));
        assert_eq!(g.arcs,
                   vec!(make_arc("0_00_data_1", VertexId(0), VertexId(1)),
                        make_arc("0_00_data_2", VertexId(0), VertexId(1)),
                        make_arc("0_01_data", VertexId(0), VertexId(2)),
                        make_arc("1_10_data", VertexId(3), VertexId(4)),
                        make_arc("1_1_data", VertexId(3), VertexId(3))));
        
        Collector::retain_reachable(&mut g, &[VertexId(0)]);
        assert_eq!(g.vertices,
                   vec!(make_vertex("0_data", vec!(), vec!(EdgeId(0), EdgeId(1), EdgeId(2))),
                        make_vertex("00_data", vec!(EdgeId(0), EdgeId(1)), vec!()),
                        make_vertex("01_data", vec!(EdgeId(2)), vec!())));
        assert_eq!(g.arcs,
                   vec!(make_arc("0_00_data_1", VertexId(0), VertexId(1)),
                        make_arc("0_00_data_2", VertexId(0), VertexId(1)),
                        make_arc("0_01_data", VertexId(0), VertexId(2))));

        let mut state_associations = HashMap::new();
        state_associations.insert("0", VertexId(0));
        state_associations.insert("00", VertexId(1));
        state_associations.insert("01", VertexId(2));
        let mut state_ids = HashIndexing::default();
        mem::swap(&mut state_ids, &mut g.state_ids);
        assert_eq!(state_ids.to_table().to_hash_map(), state_associations);
    }

    #[test]
    fn cycles_ok() {
        let mut g = empty_graph();
        g.add_edge("0", |_| "0_data", "00", |_| "00_data", "0_00_data");
        g.add_edge("00", |_| "00_data", "00", |_| "00_data", "00_00_data");
        g.add_edge("00", |_| "00_data", "0", |_| "0_data", "00_0_data");
        g.add_edge("0", |_| "0_data", "01", |_| "01_data", "0_01_data");
        g.add_edge("01", |_| "01_data", "010", |_| "010_data", "01_010_data");
        g.add_edge("root", |_| "root_data", "0", |_| "0_data", "root_0_data");
        assert_eq!(g.vertices,
                   vec!(make_vertex("0_data", vec!(EdgeId(2), EdgeId(5)), vec!(EdgeId(0), EdgeId(3))),
                        make_vertex("00_data", vec!(EdgeId(0), EdgeId(1)), vec!(EdgeId(1), EdgeId(2))),
                        make_vertex("01_data", vec!(EdgeId(3)), vec!(EdgeId(4))),
                        make_vertex("010_data", vec!(EdgeId(4)), vec!()),
                        make_vertex("root_data", vec!(), vec!(EdgeId(5)))));
        assert_eq!(g.arcs,
                   vec!(make_arc("0_00_data", VertexId(0), VertexId(1)),
                        make_arc("00_00_data", VertexId(1), VertexId(1)),
                        make_arc("00_0_data", VertexId(1), VertexId(0)),
                        make_arc("0_01_data", VertexId(0), VertexId(2)),
                        make_arc("01_010_data", VertexId(2), VertexId(3)),
                        make_arc("root_0_data", VertexId(4), VertexId(0))));

        Collector::retain_reachable(&mut g, &[VertexId(1)]);
        assert_eq!(g.vertices,
                   vec!(make_vertex("00_data", vec!(EdgeId(2), EdgeId(0)), vec!(EdgeId(0), EdgeId(1))),
                        make_vertex("0_data", vec!(EdgeId(1)), vec!(EdgeId(2), EdgeId(3))),
                        make_vertex("01_data", vec!(EdgeId(3)), vec!(EdgeId(4))),
                        make_vertex("010_data", vec!(EdgeId(4)), vec!())));
        assert_eq!(g.arcs,
                   vec!(make_arc("00_00_data", VertexId(0), VertexId(0)),
                        make_arc("00_0_data", VertexId(0), VertexId(1)),
                        make_arc("0_00_data", VertexId(1), VertexId(0)),
                        make_arc("0_01_data", VertexId(1), VertexId(2)),
                        make_arc("01_010_data", VertexId(2), VertexId(3))));

        let mut state_associations = HashMap::new();
        state_associations.insert("00", VertexId(0));
        state_associations.insert("0", VertexId(1));
        state_associations.insert("01", VertexId(2));
        state_associations.insert("010", VertexId(3));
        let mut state_ids = HashIndexing::<&'static str, VertexId>::default();
        mem::swap(&mut state_ids, &mut g.state_ids);
        assert_eq!(state_ids.to_table().to_hash_map(), state_associations);
    }
}
