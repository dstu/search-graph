mod hidden;
pub mod mutators;
pub mod nav;
pub mod search;

use std::hash::Hash;

use self::hidden::base::*;
use self::hidden::nav::{Node, make_node};
use self::hidden::mutators::{MutEdge, MutNode, make_mut_edge, make_mut_node};

/// A search graph.
///
/// Supports incremental rollout of game state topology and vertex
/// de-duplication with transposition tables. Limited support is provided for
/// deletion of components and compaction in memory after deletion.
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
    /// Lookup table that maps from game states to `VertexId`.
    state_ids: StateNamespace<T>,
    vertices: Vec<RawVertex<S>>,  // Indexed by VertexId.
    arcs: Vec<Arc<A>>,  // Indexed by EdgeId.
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

    /// Returns the vertex for the given `VertexId`.
    fn get_vertex(&self, state: VertexId) -> &RawVertex<S> {
        &self.vertices[state.as_usize()]
    }

    /// Returns the vertex for the given `VertexId`.
    fn get_vertex_mut(&mut self, state: VertexId) -> &mut RawVertex<S> {
        &mut self.vertices[state.as_usize()]
    }

    /// Returns the edge for the given `EdgeId`.
    fn get_arc(&self, arc: EdgeId) -> &Arc<A> {
        &self.arcs[arc.as_usize()]
    }

    /// Returns the edge for the given `EdgeId`.
    fn get_arc_mut(&mut self, arc: EdgeId) -> &mut Arc<A> {
        &mut self.arcs[arc.as_usize()]
    }

    /// Adds a new vertex with the given data, returning a mutable reference to it.
    ///
    /// This method does not add incoming or outgoing edges (expanded or
    /// not). That must be done by calling `add_arc` with the new vertex
    /// `VertexId`.
    fn add_vertex(&mut self, data: S) -> &mut RawVertex<S> {
        self.vertices.push(RawVertex {
            data: data,
            parents: Vec::new(),
            children: Vec::new(),
        });
        self.vertices.last_mut().unwrap()
    }

    /// Adds a new edge with the given data, source, and target. Returns the
    /// internal ID for the new edge.
    fn add_arc(&mut self, data: A, source: VertexId, target: Target<VertexId, ()>) -> EdgeId {
        let arc_id = EdgeId(self.arcs.len());
        self.get_vertex_mut(source).children.push(arc_id);
        if let Target::Expanded(target_id) = target {
            self.get_vertex_mut(target_id).parents.push(arc_id);
        }
        self.arcs.push(Arc { data: data, source: source, target: target, });
        arc_id
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

    /// Adds an edge from the vertex with state data `source` to the vertex with
    /// state data `dest`. If vertices are not found for `source` or `dest`,
    /// they are added, with the data provided by `source_data` and `dest_data`
    /// callbacks.
    ///
    /// The edge that is created will have the data `edge_data`. Returns a
    /// mutable edge handle for that edge.
    pub fn add_edge<'s, F, G>(&'s mut self, source: T, source_data: F, dest: T, dest_data: G,
                              edge_data: A) -> MutEdge<'s, T, S, A>
        where F: for<'b> FnOnce(Node<'b, T, S, A>) -> S, G: for<'b> FnOnce(Node<'b, T, S, A>) -> S {
            let source_id = match self.state_ids.get_or_insert(source) {
                NamespaceInsertion::Present(id) => id,
                NamespaceInsertion::New(id) => {
                    let data = source_data(make_node(self, id));
                    self.add_vertex(data);
                    id
                },
            };
            let dest_id = match self.state_ids.get_or_insert(dest) {
                NamespaceInsertion::Present(id) => id,
                NamespaceInsertion::New(id) => {
                    let data = dest_data(make_node(self, id));
                    self.add_vertex(data);
                    id
                },
            };
            let arc_id = self.add_arc(edge_data, source_id, Target::Expanded(dest_id));
            make_mut_edge(self, arc_id)
        }

    /// Returns the number of vertices in the graph.
    pub fn vertex_count(&self) -> usize {
        // TODO: This is actually the number of vertices we have allocated.
        self.vertices.len()
    }

    /// Returns the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        // TODO: This is actually the number of edges we have allocated.
        self.arcs.len()
    }

    /// Deletes all graph components that are not reachable by traversal
    /// starting from each vertex corresponding to the game states in `roots`.
    ///
    /// Game states in `roots` which do not have a corresponding vertex are
    /// ignored.
    pub fn retain_reachable_from(&mut self, roots: &[&T]) {
        let mut root_ids = Vec::with_capacity(roots.len());
        for state in roots.iter() {
            if let Some(id) = self.state_ids.get(state) {
                root_ids.push(id);
            }
        }
        self.retain_reachable_from_ids(&root_ids);
    }

    /// As `retain_reachable_from`, but working over raw `VertexId`s instead of
    /// root data.
    fn retain_reachable_from_ids(&mut self, root_ids: &[VertexId]) {
        self::hidden::mutators::mark_compact::Collector::retain_reachable(self, root_ids);
    }
}

/// The target of an outgoing graph edge.
///
/// A search graph is built up incrementally, and this type is used to represent
/// an edge whose destination vertex isn't yet known. Graph-modifying operations
/// which are executed while exploring the graph topology may expand such edges.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target<T, R> {
    /// Edge has not yet been expanded. Associated data may be used to perform
    /// expansion.
    Unexpanded(R),
    /// Edge has been expanded. Associated data indicates the target vertex. The
    /// target will have a backpointer to this edge's source in its parent list.
    Expanded(T),
}
