//! Read-path helpers for ThinWedge memories.
//!
//! This crate owns memory injection, memory citation parsing, and telemetry
//! classification for read access to the memory folder. It intentionally does
//! not depend on the memory write pipeline.

pub mod citations;
mod metrics;
pub mod usage;

use thinwedge_utils_absolute_path::AbsolutePathBuf;

pub fn memory_root(thinwedge_home: &AbsolutePathBuf) -> AbsolutePathBuf {
    thinwedge_home.join("memories")
}
