use std::hash::Hash;

use ::{Graph, Target};
use ::hidden::base::{Arc, ArcId, StateId, Vertex};

/// Immutable handle to a graph vertex, called a "node handle."
///
/// This zipper-like type enables traversal of a graph along the vertex's
/// incoming and outgoing edges.
pub struct Node<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    graph: &'a Graph<T, S, A>,
    id: StateId,
}

pub fn make_node<'a, T, S, A>(graph: &'a Graph<T, S, A>, id: StateId) -> Node<'a, T, S, A>
    where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
        Node { graph: graph, id: id, }
    }

impl<'a, T, S, A> Node<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    fn children(&self) -> &'a [ArcId] {
        &self.graph.get_vertex(self.id).children
    }

    /// Returns an immutable ID that is guaranteed to identify this vertex
    /// uniquely within its graph. This ID may change when the graph is mutated.
    pub fn get_id(&self) -> usize {
        self.id.as_usize()
    }

    fn parents(&self) -> &'a [ArcId] {
        &self.graph.get_vertex(self.id).parents
    }

    /// Returns the data at this vertex.
    pub fn get_data(&self) -> &'a S {
        &self.graph.get_vertex(self.id).data
    }

    /// Returns true iff this vertex has no outgoing edges (regardless of
    /// whether they are expanded).
    pub fn is_leaf(&self) -> bool {
        self.children().is_empty()
    }

    /// Returns true iff this vertex has no incoming edges.
    pub fn is_root(&self) -> bool {
        self.parents().is_empty()
    }

    /// Returns a traversible list of outgoing edges.
    pub fn get_child_list(&self) -> ChildList<'a, T, S, A> {
        ChildList { graph: self.graph, id: self.id, }
    }

    /// Returns a traversible list of incoming edges.
    pub fn get_parent_list(&self) -> ParentList<'a, T, S, A> {
        ParentList { graph: self.graph, id: self.id, }
    }
}

/// A traversible list of a vertex's outgoing edges.
pub struct ChildList<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    graph: &'a Graph<T, S, A>,
    id: StateId,
}

pub fn make_child_list<'a, T, S, A>(graph: &'a Graph<T, S, A>, id: StateId) -> ChildList<'a, T, S, A>
    where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
        ChildList { graph: graph, id: id, }
    }

impl<'a, T, S, A> ChildList<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    fn vertex(&self) -> &'a Vertex<S> {
        self.graph.get_vertex(self.id)
    }

    /// Returns the number of edges.
    pub fn len(&self) -> usize {
        self.vertex().children.len()
    }

    /// Returns true iff there are no outgoing edges.
    pub fn is_empty(&self) -> bool {
        self.vertex().children.is_empty()
    }

    /// Returns a node handle for the vertex these edges originate from.
    pub fn get_source_node(&self) -> Node<'a, T, S, A> {
        Node { graph: self.graph, id: self.id, }
    }

    /// Returns an edge handle for the `i`th edge.
    pub fn get_edge(&self, i: usize) -> Edge<'a, T, S, A> {
        Edge { graph: self.graph, id: self.vertex().children[i], }
    }
}

/// A traversible list of a vertex's incoming edges.
pub struct ParentList<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    graph: &'a Graph<T, S, A>,
    id: StateId,
}

pub fn make_parent_list<'a, T, S, A>(graph: &'a Graph<T, S, A>, id: StateId) -> ParentList<'a, T, S, A>
    where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
        ParentList { graph: graph, id: id, }
    }

impl<'a, T, S, A> ParentList<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    fn vertex(&self) -> &'a Vertex<S> {
        self.graph.get_vertex(self.id)
    }

    /// Returns the number of edges.
    pub fn len(&self) -> usize {
        self.vertex().parents.len()
    }

    /// Returns true iff there are no incoming edges.
    pub fn is_empty(&self) -> bool {
        self.vertex().parents.is_empty()
    }

    /// Returns a node handle for the vertex these edges point to.
    pub fn target_node(&self) -> Node<'a, T, S, A> {
        Node { graph: self.graph, id: self.id, }
    }

    /// Returns an edge handle for the `i`th edge.
    pub fn get_edge(&self, i: usize) -> Edge<'a, T, S, A> {
        Edge { graph: self.graph, id: self.vertex().parents[i] }
    }
}

/// Immutable handle to a graph edge, called an "edge handle."
///
/// This zipper-like type enables traversal of a graph along the edge's source
/// and target vertices.
pub struct Edge<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    graph: &'a Graph<T, S, A>,
    id: ArcId,
}

pub fn make_edge<'a, T, S, A>(graph: &'a Graph<T, S, A>, id: ArcId) -> Edge<'a, T, S, A>
    where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
        Edge { graph: graph, id: id, }
    }

impl<'a, T, S, A> Edge<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    fn arc(&self) -> &'a Arc<A> {
        self.graph.get_arc(self.id)
    }

    /// Returns an immutable ID that is guaranteed to identify this edge
    /// uniquely within its graph.  This ID may change when the graph is
    /// mutated.
    pub fn get_id(&self) -> usize {
        self.id.as_usize()
    }

    /// Returns the data at this edge.
    pub fn get_data(&self) -> &'a A {
        &self.arc().data
    }

    /// Returns a node handle for this edge's source vertex.
    pub fn get_source(&self) -> Node<'a, T, S, A> {
        Node { graph: self.graph, id: self.arc().source, }
    }

    /// Returns a node handle for this edge's target vertex.
    pub fn get_target(&self) -> Target<Node<'a, T, S, A>, ()> {
        match self.arc().target {
            Target::Cycle(id) => Target::Cycle(Node { graph: self.graph, id: id, }),
            Target::Unexpanded(_) => Target::Unexpanded(()),
            Target::Expanded(id) => Target::Expanded(Node { graph: self.graph, id: id, }),
        }
    }
}
