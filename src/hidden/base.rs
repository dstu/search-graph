use ::symbol_table;

/// Internal edge identifier.
///
/// This type is not exported by the crate because it does not identify the
/// graph that it belongs to, which makes it only slightly less dangerous than a
/// pointer with no lifetime.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EdgeId(pub usize);

impl EdgeId {
    /// Converts an `EdgeId` to a usize that is guaranteed to be unique within a
    /// graph.
    pub fn as_usize(self) -> usize {
        let EdgeId(x) = self;
        x
    }
}

/// Internal vertex identifier.
///
/// For a given graph, distinct `VertexId`s are associated with distinct game
/// states. This type is not exported by the crate because it does not identify
/// the graph that it belongs to, which makes it only slightly less dangerous
/// than a pointer with no lifetime.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VertexId(pub usize);

impl Default for VertexId {
    fn default() -> Self { VertexId(0) }
}

impl symbol_table::SymbolId for VertexId {
    fn next(&self) -> Self { VertexId(self.0 + 1) }
    fn as_usize(&self) -> usize { self.0 }
}

/// Internal type for graph edges.
///
/// The Hash, Ord, and Eq implementations will conflate parallel edges with
/// identical statistics.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RawEdge<A> {
    /// Edge data.
    pub data: A,
    /// Source vertex.
    pub source: VertexId,
    /// Target vertex.
    pub target: VertexId,
}

/// Internal type for graph vertices.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RawVertex<S> {
    /// Vertex data.
    pub data: S,
    /// Parent edges pointing into this vertex.
    pub parents: Vec<EdgeId>,
    /// Child edges pointing out of this vertex.
    pub children: Vec<EdgeId>,
}
