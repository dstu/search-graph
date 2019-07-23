pub(crate) mod base;
pub mod mutators;
pub mod nav;
pub mod search;
pub mod view;

use std::hash::Hash;

use base::{EdgeId, RawEdge, RawVertex, VertexId};
use symbol_map::indexing::{Indexing, Insertion};
use symbol_map::SymbolId;

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
/// handle with `find_node`. To modify graph contents, add new root vertices with
/// `add_node` and retrieve extant vertices with `find_node_mut`.
pub struct Graph<T: Hash + Eq + Clone, S, A> {
  /// Lookup table that maps from game states to `VertexId`.
  state_ids: symbol_map::indexing::HashIndexing<T, VertexId>,
  vertices: Vec<RawVertex<S>>, // Indexed by VertexId.
  arcs: Vec<RawEdge<A>>,       // Indexed by EdgeId.
}

impl<T: Hash + Eq + Clone, S, A> Graph<T, S, A> {
  /// Creates an empty `Graph` with no vertices or edges.
  pub fn new() -> Self {
    Graph {
      state_ids: Default::default(),
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
  fn get_arc(&self, arc: EdgeId) -> &RawEdge<A> {
    &self.arcs[arc.as_usize()]
  }

  /// Returns the edge for the given `EdgeId`.
  fn get_arc_mut(&mut self, arc: EdgeId) -> &mut RawEdge<A> {
    &mut self.arcs[arc.as_usize()]
  }

  /// Returns the game state associated with `id`.
  fn get_state(&self, id: VertexId) -> Option<&T> {
    self.state_ids.get_symbol(&id).as_ref().map(|x| x.data())
  }

  /// Adds a new vertex with the given data, returning a mutable reference to it.
  ///
  /// This method does not add incoming or outgoing edges (expanded or
  /// not). That must be done by calling `add_arc` with the new vertex
  /// `VertexId`.
  fn add_raw_vertex(&mut self, data: S) -> &mut RawVertex<S> {
    self.vertices.push(RawVertex {
      data: data,
      parents: Vec::new(),
      children: Vec::new(),
    });
    self.vertices.last_mut().unwrap()
  }

  /// Adds a new edge with the given data, source, and target. Returns the
  /// internal ID for the new edge.
  fn add_raw_edge(&mut self, data: A, source: VertexId, target: VertexId) -> EdgeId {
    let arc_id = EdgeId(self.arcs.len());
    self.get_vertex_mut(source).children.push(arc_id);
    self.get_vertex_mut(target).parents.push(arc_id);
    self.arcs.push(RawEdge {
      data: data,
      source: source,
      target: target,
    });
    arc_id
  }

  /// Gets a node handle for the given game state.
  ///
  /// If `state` does not correspond to a known game state, returns `None`.
  pub fn find_node<'s>(&'s self, state: &T) -> Option<nav::Node<'s, T, S, A>> {
    match self.state_ids.get(state) {
      Some(symbol) => Some(nav::Node::new(self, *symbol.id())),
      None => None,
    }
  }

  /// Gets a mutable node handle for the given game state.
  ///
  /// If `state` does not correspond to a known game state, returns `None`.
  pub fn find_node_mut<'s>(&'s mut self, state: &T) -> Option<mutators::MutNode<'s, T, S, A>> {
    match self.state_ids.get(state).map(|s| s.id().clone()) {
      Some(id) => Some(mutators::MutNode::new(self, id)),
      None => None,
    }
  }

  /// Adds a vertex (with no parents or children) for the given game state and
  /// data and returns a mutable handle for it.
  ///
  /// If `state` is already known, returns a mutable handle to that state,
  /// ignoring the `data` parameter. As a result, this method is guaranteed to
  /// return a handle for a root vertex only when `state` is a novel game
  /// state.
  pub fn add_node<'s>(&'s mut self, state: T, data: S) -> mutators::MutNode<'s, T, S, A> {
    let node_id = match self.state_ids.get_or_insert(state).map(|s| s.id().clone()) {
      Insertion::Present(id) => id,
      Insertion::New(id) => {
        self.add_raw_vertex(data);
        id
      }
    };
    mutators::MutNode::new(self, node_id)
  }

  /// Adds an edge from the vertex with state data `source` to the vertex with
  /// state data `dest`. If vertices are not found for `source` or `dest`,
  /// they are added, with the data provided by `source_data` and `dest_data`
  /// callbacks.
  ///
  /// The edge that is created will have the data `edge_data`. Returns a
  /// mutable edge handle for that edge.
  pub fn add_edge<'s, F, G>(
    &'s mut self,
    source: T,
    source_data: F,
    dest: T,
    dest_data: G,
    edge_data: A,
  ) -> mutators::MutEdge<'s, T, S, A>
  where
    F: for<'b> FnOnce(nav::Node<'b, T, S, A>) -> S,
    G: for<'b> FnOnce(nav::Node<'b, T, S, A>) -> S,
  {
    let source_id = match self.state_ids.get_or_insert(source).map(|s| s.id().clone()) {
      Insertion::Present(id) => id,
      Insertion::New(id) => {
        let data = source_data(nav::Node::new(self, id));
        self.add_raw_vertex(data);
        id
      }
    };
    let dest_id = match self.state_ids.get_or_insert(dest).map(|s| s.id().clone()) {
      Insertion::Present(id) => id,
      Insertion::New(id) => {
        let data = dest_data(nav::Node::new(self, id));
        self.add_raw_vertex(data);
        id
      }
    };
    let edge_id = self.add_raw_edge(edge_data, source_id, dest_id);
    mutators::MutEdge::new(self, edge_id)
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
}

#[cfg(test)]
mod test {
  use crossbeam_utils::thread;
  use std::sync::Arc;

  type Graph = crate::Graph<&'static str, &'static str, &'static str>;

  #[test]
  fn send_to_thread_safe_ok() {
    let mut g = Graph::new();
    g.add_edge("root", |_| "root_data", "0", |_| "0_data", "root_0_data");
    g.add_edge("root", |_| "root_data", "1", |_| "1_data", "root_1_data");
    let graph = Arc::new(g);
    thread::scope(move |s| {
      let g = graph.clone();
      let t1 = s.spawn(move |_| g.find_node(&"root").map(|n| n.get_id()));
      let g = graph.clone();
      let t2 = s.spawn(move |_| g.find_node(&"1").map(|n| n.get_id()));
      match t1.join() {
        Ok(Some(id)) => assert_eq!(id, 0),
        _ => panic!(),
      }
      match t2.join() {
        Ok(Some(id)) => assert_eq!(id, 2),
        _ => panic!(),
      }
    })
    .unwrap();
  }

  #[test]
  fn sync_to_thread_ok() {
    let mut g = Graph::new();
    g.add_edge("root", |_| "root_data", "0", |_| "0_data", "root_0_data");
    g.add_edge("root", |_| "root_data", "1", |_| "1_data", "root_1_data");
    let g = &g;
    thread::scope(|s| {
      let t1 = s.spawn(move |_| g.find_node(&"root").map(|n| n.get_id()));
      let t2 = s.spawn(move |_| g.find_node(&"1").map(|n| n.get_id()));
      match t1.join() {
        Ok(Some(id)) => assert_eq!(id, 0),
        _ => panic!(),
      }
      match t2.join() {
        Ok(Some(id)) => assert_eq!(id, 2),
        _ => panic!(),
      }
    })
    .unwrap();
  }
}
