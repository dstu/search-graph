mod hidden;

use std::hash::Hash;

use self::hidden::base::*;
use self::hidden::nav::make_node;
use self::hidden::mutators::make_mut_node;

pub use self::hidden::nav::{ChildList, Edge, Node, ParentList};
pub use self::hidden::mutators::{EdgeExpander, MutChildList, MutEdge, MutNode, MutParentList};

/// A search graph.
///
/// Supports incremental rollout of game state topology, vertex de-duplication
/// with transposition tables, and cycle detection. Does not support deletion.
///
/// - `T`: The type of game states. It is required to derive `Hash` and `Eq` to
///   so that it may be stored in a hashtable, where game states are looked up to
///   support de-duplication of game states. It is required to derive `Clone` to
///   accommodate a limitation of the `HashMap` interface.
/// - `S`: The type of graph vertex data.
/// - `A`: The type of graph edge data.
///
/// Vertices are addressed by content. To examine graph contents, obtain a node
/// handle with `get_node`. To modify graph contents, add new root vertices with
/// `add_root` and retrieve extant vertices with `get_node_mut`.
pub struct Graph<T, S, A> where T: Hash + Eq + Clone {
    /// Lookup table that maps from game states to `StateId`.
    state_ids: StateNamespace<T>,
    vertices: Vec<Vertex<S>>,  // Indexed by StateId.
    arcs: Vec<Arc<A>>,  // Indexed by ArcId.
}

impl<T, S, A> Graph<T, S, A> where T: Hash + Eq + Clone {
    /// Creates an empty `Graph` with no vertices or edges.
    pub fn new() -> Self {
        Graph {
            state_ids: StateNamespace::new(),
            vertices: Vec::new(),
            arcs: Vec::new(),
        }
    }

    /// Returns the vertex for the given `StateId`.
    fn get_vertex(&self, state: StateId) -> &Vertex<S> {
        &self.vertices[state.as_usize()]
    }

    /// Returns the vertex for the given `StateId`.
    fn get_vertex_mut(&mut self, state: StateId) -> &mut Vertex<S> {
        &mut self.vertices[state.as_usize()]
    }

    /// Returns the edge for the given `ArcId`.
    fn get_arc(&self, arc: ArcId) -> &Arc<A> {
        &self.arcs[arc.as_usize()]
    }

    /// Returns the edge for the given `ArcId`.
    fn get_arc_mut(&mut self, arc: ArcId) -> &mut Arc<A> {
        &mut self.arcs[arc.as_usize()]
    }

    /// Adds a new vertex with the given data, returning a mutable reference to it.
    ///
    /// This method does not add incoming or outgoing edges (expanded or
    /// not). That must be done by calling `add_arc` with the new vertex
    /// `StateId`.
    fn add_vertex(&mut self, data: S) -> &mut Vertex<S> {
        self.vertices.push(Vertex { data: data, parents: Vec::new(), children: Vec::new(), });
        self.vertices.last_mut().unwrap()
    }

    /// Adds a new edge with the given data, source, and target.
    ///
    /// Iff `target` is `Target::Expanded(id)`, the vertex with `StateId` of
    /// `id` will have the vertex `source` added as a parent.
    fn add_arc(&mut self, data: A, source: StateId, target: Target<StateId, ()>) {
        let arc = Arc { data: data, source: source, target: target, };
        let arc_id = ArcId(self.arcs.len());
        if let Target::Expanded(target_id) = target {
            self.get_vertex_mut(target_id).parents.push(arc_id);
        }
        self.get_vertex_mut(source).children.push(arc_id);
        self.arcs.push(arc);
    }

    /// Checks whether a path exists from `source` to `target`.
    ///
    /// Paths are found using a simple depth-first search. This method only
    /// follows arcs with destination type `Target::Expanded`, so it does not
    /// find paths that go through an ancestor of `source`.
    fn path_exists(&self, source: StateId, target: StateId) -> bool {
        let mut frontier = vec![source];
        while !frontier.is_empty() {
            let state = frontier.pop().unwrap();
            if target == state {
                return true
            }
            for arc_id in &self.get_vertex(state).children {
                let arc = self.get_arc(*arc_id);
                if let Target::Expanded(target_id) = arc.target {
                    frontier.push(target_id);
                }
            }
        }
        false
    }

    /// Gets a node handle for the given game state.
    ///
    /// If `state` does not correspond to a known game state, returns `None`.
    pub fn get_node<'s>(&'s self, state: &T) -> Option<Node<'s, T, S, A>> {
        match self.state_ids.get(&state) {
            Some(id) => Some(make_node(self, id)),
            None => None,
        }
    }

    /// Gets a mutable node handle for the given game state.
    ///
    /// If `state` does not correspond to a known game state, returns `None`.
    pub fn get_node_mut<'s>(&'s mut self, state: &T) -> Option<MutNode<'s, T, S, A>> {
        match self.state_ids.get(state) {
            Some(id) => Some(make_mut_node(self, id)),
            None => None,
        }
    }

    /// Adds a root vertex (one with no parents) for the given game state and
    /// data and returns a mutable handle for it.
    ///
    /// If `state` is already known, returns a mutable handle to that state,
    /// ignoring the `data` parameter. As a result, this method is guaranteed to
    /// return a handle for a root vertex only when `state` is a novel game
    /// state.
    pub fn add_root<'s>(&'s mut self, state: T, data: S) -> MutNode<'s, T, S, A> {
        let node_id = match self.state_ids.get_or_insert(state) {
            NamespaceInsertion::Present(id) => id,
            NamespaceInsertion::New(id) => {
                self.add_vertex(data);
                id
            },
        };
        make_mut_node(self, node_id)
    }
}

/// The target of an outgoing graph edge.
///
/// A search graph is built up incrementally. Any vertices are typically added
/// with all of their edges in the unexpanded state. Graph-modifying operations
/// which are executed while exploring the game state topology will expand these
/// edges. Cycle detection is done at edge expansion time.
#[derive(Clone, Copy, Debug)]
pub enum Target<T, R> {
    /// Edge has not yet been expanded.
    Unexpanded(R),
    /// Edge has been expanded but leads to a cycle. Because cycle detection is
    /// done at edge expansion time, this usually means that another edge, which
    /// was expanded previously, has the value `Target::Expanded` and points to
    /// the same vertex. The target of this edge will not have a backpointer to
    /// this edge's source in its parent list.
    Cycle(T),
    /// Edge has been expanded and was the expanded edge that lead to the game
    /// state which it points to. The target of this edge will have a
    /// backpointer to this edge's source in its parent list.
    Expanded(T),
}
