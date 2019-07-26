# Overview

This package provides a rollout-based graphical data structure. Its intended use
is building game state graphs for general game-playing or similar AI tasks.

Vertices correspond to game states. They are addressed by unique game states
(which may be de-duplicated as if by transposition table by implementing
=std::hash::Hash= and =std::cmp::Eq= appropriately for your game state type).

Edges correspond to game-modifying moves. They are bidirectional, so paths may
be traced up and down the graph.

Three different interfaces are provided to make it easy to navigate the graph
and perform updates to it.

Graph topology may be grown by adding vertices and edges. A graph may be
garbage-collected by pruning all elements that cannot be reached from an
arbitrary set of vertices.

# Copyright

Copyright 2015-2016, Donald S. Black.

Licensed under the Apache License, Version 2.0 (the “License”); you may not use
this file except in compliance with the License. You may obtain a copy of the
License at http://www.apache.org/licenses/LICENSE-2.0.

Unless required by applicable law or agreed to in writing, software distributed
under the License is distributed on an “AS IS” BASIS, WITHOUT WARRANTIES OR
CONDITIONS OF ANY KIND, either express or implied. See the License for the
specific language governing permissions and limitations under the License.
