//! Mark-and-sweep garbage collection for pruning graphs.
//!
//! This module implements a mark-and-sweep garbage collector that can prune a
//! graph so that only components reachable from a set of root game states are
//! retained. Running time and memory required are linear in graph size,
//! although there is a potential for a high cost when rebuilding the hashtable
//! that maps from game states to their IDs.

use ::{Graph, Target};
use ::hidden::base::{ArcId, StateId, StateNamespace};

use std::cmp::Eq;
use std::collections::VecDeque;
use std::hash::Hash;

/// Garbage collector state. Use the `clean()` method to prune a graph.
pub struct Collector<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    graph: &'a mut Graph<T, S, A>,
    marked_state_count: usize,
    marked_arc_count: usize,
    state_id_map: Vec<Option<StateId>>,
    arc_id_map: Vec<Option<ArcId>>,
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
        c.sweep_vertices();
        c.sweep_arcs();
        c.remap_state_namespace();
    }

    /// Creates a new mark-and-sweep garbage collector with empty initial state.
    fn new(graph: &'a mut Graph<T, S, A>) -> Self {
        let empty_state_ids = vec![None; graph.vertices.len()];
        let empty_arc_ids = vec![None; graph.arcs.len()];
        Collector {
            graph: graph,
            marked_state_count: 0,
            marked_arc_count: 0,
            state_id_map: empty_state_ids,
            arc_id_map: empty_arc_ids,
        }
    }

    /// Traverses graph components reachable from `roots` and marks them as
    /// reachable. Also builds a new graph component addressing scheme that
    /// reassigns `StateId` and `ArcId` values.
    ///
    /// As side effects, arc sources and vertex children are updated to use the
    /// new addressing scheme.
    fn mark(&mut self, roots: &[StateId]) {
        let mut frontier = VecDeque::new();
        for id in roots.iter() {
            frontier.push_back(*id);
        }
        loop {
            match frontier.pop_front() {
                None => break,
                Some(state_id) => {
                    let (new_state_id, mut child_arc_ids) = {
                        let vertex = self.graph.get_vertex_mut(state_id);
                        if vertex.mark {
                            continue
                        }
                        // Mark all reachable vertices.
                        vertex.mark = true;
                        let new_state_id = {
                            let new_state_id = StateId(self.marked_state_count);
                            self.marked_state_count += 1;
                            new_state_id
                        };
                        self.state_id_map[state_id.as_usize()] = Some(new_state_id);
                        (new_state_id, vertex.children.clone())
                    };

                    for arc_id in child_arc_ids.iter_mut() {
                        let arc = self.graph.get_arc_mut(*arc_id);
                        if arc.mark {
                            continue
                        }
                        // Mark all reachable arcs.
                        arc.mark = true;
                        // Update arc sources to use new state IDs.
                        arc.source = new_state_id;
                        if let Target::Expanded(child_vertex_id) = arc.target {
                            frontier.push_front(child_vertex_id);
                        }
                        let new_arc_id = {
                            let new_arc_id = ArcId(self.marked_arc_count);
                            self.marked_arc_count += 1;
                            new_arc_id
                        };
                        self.arc_id_map[arc_id.as_usize()] = Some(new_arc_id);
                        *arc_id = new_arc_id;
                    }
                    // Update vertex children to use new ArcIds.
                    self.graph.get_vertex_mut(state_id).children = child_arc_ids;
                },
            }
        }
    }

    /// Drops vertices which were not reached in the previous `mark()`. Must be
    /// run after `mark()`.
    ///
    /// Also, resets the mark state on all vertices and updates vertex pointers
    /// to parent edges to use the new `ArcId` addressing scheme built in the
    /// previous call to `mark()`.
    fn sweep_vertices(&mut self) {
        let mut new_vertices = Vec::with_capacity(self.graph.vertices.len());
        for mut vertex in self.graph.vertices.drain(0..) {
            if !vertex.mark {
                continue
            }
            // Unmark marked vertices.
            vertex.mark = false;
            // Update vertex parents to use new ArcIds.
            for parent_arc_id in vertex.parents.iter_mut() {
                *parent_arc_id = self.arc_id_map[parent_arc_id.as_usize()].unwrap();
            }
            new_vertices.push(vertex);
        }
        new_vertices.shrink_to_fit();
        // Retain only marked vertices.
        self.graph.vertices = new_vertices;
    }

    /// Drops arcs which were not reached in the previous `mark()`. Must be run
    /// after `mark()`.
    ///
    /// Also, resets the mark state on all arcs and updates arc targets to use
    /// the new `StateId` addressing scheme built in the previous call to
    /// `mark()`.
    fn sweep_arcs(&mut self) {
        let mut new_arcs = Vec::with_capacity(self.graph.arcs.len());
        for mut arc in self.graph.arcs.drain(0..) {
            if !arc.mark {
                continue
            }
            // Unmark marked arcs.
            arc.mark = false;
            // Update arc targets to use new StateIds.
            arc.target = match arc.target {
                Target::Expanded(old_arc_id) =>
                    Target::Expanded(self.state_id_map[old_arc_id.as_usize()].unwrap()),
                Target::Unexpanded(()) =>
                    Target::Unexpanded(()),
            };
            new_arcs.push(arc);
        }
        new_arcs.shrink_to_fit();
        // Retain only marked arcs.
        self.graph.arcs = new_arcs;
    }

    /// Remaps associations between game states to use post-sweep `StateId`
    /// associations. Must be run after `mark()`.
    fn remap_state_namespace(&mut self) {
        Self::do_remap(&mut self.graph.state_ids, &self.state_id_map);
    }

    /// Private method for remapping StateIds. Borrow rules prevent us from invoking
    /// remap directly in `remap_state_namespace`.
    fn do_remap(state_ids: &mut StateNamespace<T>, state_id_map: &[Option<StateId>]) {
        state_ids.remap(|_, old_state_id| state_id_map[old_state_id.as_usize()]);
    }
}
