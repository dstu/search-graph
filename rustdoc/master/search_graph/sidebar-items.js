initSidebarItems({"enum":[["PathItem","Sum type for path elements. All elements except the head are represented with the `PathItem::Item` variant."],["SearchError","Errors that may arise during search."],["Target","The target of an outgoing graph edge."],["Traversal","Indicates which edge of a vertex to traverse. Edges are denoted by a 0-based index. This type is used by functions provided during graph search to indicate which child or parent edges to traverse."]],"struct":[["ChildList","A traversible list of a vertex's outgoing edges."],["ChildListIter","Iterator over a vertex's child edges."],["Edge","Immutable handle to a graph edge (\"edge handle\")."],["EdgeExpander","Modifies graph topology by connecting an unexpanded edge to its target vertex."],["Graph","A search graph."],["MutChildList","A traversible list of a vertex's outgoing edges."],["MutEdge","Mutable handle to a graph edge (\"edge handle\")."],["MutNode","Mutable handle to a graph vertex (\"node handle\")."],["MutParentList","A traversible list of a vertex's incoming edges."],["Node","Immutable handle to a graph vertex (\"node handle\")."],["ParentList","A traversible list of a vertex's incoming edges."],["ParentListIter","Iterator over a vertex's parent edges."],["SearchPath","Tracks the path through a graph that is followed when performing local search."],["SearchPathIter","Iterates over elements of a search path, in the order in which they were traversed, ending with the head."]]});