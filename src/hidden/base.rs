use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash::Hash;

use ::Target;

/// Internal edge identifier.
///
/// This type is not exported by the crate because it does not identify the
/// graph that it belongs to, which makes it only slightly less dangerous than a
/// pointer with no lifetime.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ArcId(pub usize);

impl ArcId {
    /// Converts an `ArcId` to a usize that is guaranteed to be unique within a
    /// graph.
    pub fn as_usize(self) -> usize {
        let ArcId(x) = self;
        x
    }
}

/// Internal vertex identifier.
///
/// For a given graph, distinct `StateId`s are associated with distinct game
/// states. This type is not exported by the crate because it does not identify
/// the graph that it belongs to, which makes it only slightly less dangerous
/// than a pointer with no lifetime.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StateId(pub usize);

impl StateId {
    /// Converts a `StateId` to a usize that is guaranteed to be unique within a
    /// graph.
    pub fn as_usize(self) -> usize {
        let StateId(x) = self;
        x
    }
}

/// Retains a mapping from game states to distinct IDs.
///
/// The game state type `T` is required to derive from `Clone` to accommodate a
/// limitation of the `HashMap` interface.
pub struct StateNamespace<T> where T: Hash + Eq + Clone {
    states: HashMap<T, StateId>,
}

/// The result of inserting a game state into a `StateNamespace`.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum NamespaceInsertion {
    /// State was already present, with the given ID.
    Present(StateId),
    /// State was new, and is inserted with the given ID.
    New(StateId),
}

impl<T> StateNamespace<T> where T: Hash + Eq + Clone {
    /// Creates a new, empty `StateNamespace`.
    pub fn new() -> Self {
        StateNamespace {
            states: HashMap::new(),
        }
    }

    /// Retrieves a `StateId` for `state`, creating a new one if necessary.
    ///
    /// This may insert a new state into `self` or simply retrieve the `StateId`
    /// associated with it in a prior insertion operation.
    pub fn get_or_insert(&mut self, state: T) -> NamespaceInsertion {
        let next_state_id = StateId(self.states.len());
        match self.states.entry(state) {
            Entry::Occupied(e) => NamespaceInsertion::Present(*e.get()),
            Entry::Vacant(e) => NamespaceInsertion::New(*e.insert(next_state_id)),
        }
    }

    /// Retrieves a `StateId` for `state`.
    ///
    /// If `state` has not been inserted, returns `None`.
    pub fn get(&self, state: &T) -> Option<StateId> {
        self.states.get(state).map(|x| *x)
    }
}

/// Internal type for graph edges.
#[derive(Debug)]
pub struct Arc<A> {
    /// Edge data.
    pub data: A,
    /// Source vertex.
    pub source: StateId,
    /// Target vertex. If this arc is unexpanded, it is
    /// `Target::Unexpanded(())`; otherwise, it is either `Target::Cycle(id)` or
    /// `Target::Expanded(id)` for target vertex with a `StateId` of `id`.
    pub target: Target<StateId, ()>,
}

/// Internal type for graph vertices.
#[derive(Debug)]
pub struct Vertex<S> {
    /// Vertex data.
    pub data: S,
    /// Parent edges pointing into this vertex.
    pub parents: Vec<ArcId>,
    /// Child edges pointing out of this vertex.
    pub children: Vec<ArcId>,
}
