use std::cmp::Eq;

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

    // /// Changes associations between states and `VertexId`s.
    // ///
    // /// `(T, VertexId)` associations for which `f` returns `Some(new_id)` will be
    // /// remapped to use `new_id`.
    // ///
    // /// `(T, VertexId)` associations for which `f` returns `None` will be dropped.
    // ///
    // /// It is the responsibility of the caller to ensure that states map to
    // /// unique `VertexId`s.
    // pub fn remap<F>(&mut self, mut f: F) where F: FnMut(&T, VertexId) -> Option<VertexId> {
    //     let mut new_state_to_id = HashMap::with_capacity(self.state_to_id.len());
    //     for (state, old_state_id) in self.state_to_id.drain() {
    //         if let Some(new_state_id) = f(&state, old_state_id) {
    //             new_state_to_id.insert(state.clone(), new_state_id);
    //             self.id_to_state[new_state_id.as_usize()] = state;
    //         }
    //     }
    //     new_state_to_id.shrink_to_fit();
    //     self.state_to_id = new_state_to_id;
    //     self.id_to_state.truncate(self.state_to_id.len());
    // }

    // /// Transforms this namespace mapping to a hashtable. This is intended for
    // /// testing purposes only.
    // #[cfg(test)]
    // pub fn to_hash_map(self) -> HashMap<Rc<T>, VertexId> {
    //     self.state_to_id
    // }

    // pub fn get_state(&self, id: usize) -> Option<&T> {
    //     self.id_to_state.get(id).map(|x| &**x)
    // }

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
