//! Shortest paths on Dushanbe's real roads, for the S4 routing race: Dijkstra, A* and
//! Contraction Hierarchies on the same graph, compiled to WebAssembly and timed in the browser.

pub mod graph;
pub mod search;

#[cfg(feature = "prep")]
pub mod prep;

#[cfg(target_arch = "wasm32")]
mod wasm;

pub use graph::{Graph, NONE};
pub use search::{Algo, Search, Step};

#[cfg(test)]
mod tests;
