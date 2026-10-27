//! Shared LUPS particle curves, effect lifetimes and reusable GPU streams.
//! Uploads happen at birth; the vertex shader evaluates age and billboards.
mod batch;
mod emit;
pub mod geometry;
mod gpu;
mod model;

pub use batch::Batch;
pub use emit::emit_effect;
pub use gpu::Gpu;
pub use model::*;
