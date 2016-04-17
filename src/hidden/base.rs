use std::cmp::Eq;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash::Hash;

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

impl VertexId {
    /// Converts a `VertexId` to a usize that is guaranteed to be unique within a
    /// graph.
    pub fn as_usize(self) -> usize {
        let VertexId(x) = self;
        x
    }
}

/// Retains a mapping from game states to distinct IDs.
///
/// The game state type `T` is required to derive from `Clone` to accommodate a
/// limitation of the `HashMap` interface.
pub struct StateNamespace<T> where T: Hash + Eq + Clone {
    states: HashMap<T, VertexId>,
}

/// The result of inserting a game state into a `StateNamespace`.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum NamespaceInsertion {
    /// State was already present, with the given ID.
    Present(VertexId),
    /// State was new, and is inserted with the given ID.
    New(VertexId),
}

impl<T> StateNamespace<T> where T: Hash + Eq + Clone {
    /// Creates a new, empty `StateNamespace`.
    pub fn new() -> Self {
        StateNamespace { states: HashMap::new(), }
    }

    /// Retrieves a `VertexId` for `state`, creating a new one if necessary.
    ///
    /// This may insert a new state into `self` or simply retrieve the `VertexId`
    /// associated with it in a prior insertion operation.
    pub fn get_or_insert(&mut self, state: T) -> NamespaceInsertion {
        let next_state_id = VertexId(self.states.len());
        match self.states.entry(state) {
            Entry::Occupied(e) => NamespaceInsertion::Present(*e.get()),
            Entry::Vacant(e) => NamespaceInsertion::New(*e.insert(next_state_id)),
        }
    }

    /// Retrieves a `VertexId` for `state`.
    ///
    /// If `state` has not been inserted, returns `None`.
    pub fn get(&self, state: &T) -> Option<VertexId> {
        self.states.get(state).map(|x| *x)
    }

    /// Changes associations between states and `VertexId`s.
    ///
    /// `(T, VertexId)` associations for which `f` returns `Some(new_id)` will be
    /// remapped to use `new_id`.
    ///
    /// `(T, VertexId)` associations for which `f` returns `None` will be dropped.
    ///
    /// It is the responsibility of the caller to ensure that states map to
    /// unique `VertexId`s.
    pub fn remap<F>(&mut self, mut f: F) where F: FnMut(&T, VertexId) -> Option<VertexId> {
        let mut new_states = HashMap::with_capacity(self.states.len());
        for (state, old_state_id) in self.states.drain() {
            if let Some(new_state_id) = f(&state, old_state_id) {
                new_states.insert(state, new_state_id);
            }
        }
        new_states.shrink_to_fit();
        self.states = new_states;
    }

    /// Transforms this namespace mapping to a hashtable. This is intended for
    /// testing purposes only.
    #[cfg(test)]
    pub fn to_hash_map(self) -> HashMap<T, VertexId> {
        self.states
    }
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