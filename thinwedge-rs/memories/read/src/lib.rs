//! Read-path helpers for ThinWedge memories.
//!
//! This crate owns memory injection, memory citation parsing, and telemetry
//! classification for read access to the memory folder. It intentionally does
//! not depend on the memory write pipeline.

pub mod citations;
mod metrics;
mod prompts;
pub mod usage;

use thinwedge_utils_absolute_path::AbsolutePathBuf;

pub use prompts::build_memory_tool_developer_instructions;

const MEMORY_TOOL_DEVELOPER_INSTRUCTIONS_SUMMARY_TOKEN_LIMIT: usize = 5_000;

pub fn memory_root(thinwedge_home: &AbsolutePathBuf) -> AbsolutePathBuf {
    thinwedge_home.join("memories")
}

/// Returns true when a memory path is safe to expose through memory read/list/search surfaces.
pub fn is_public_memory_path(path: &std::path::Path) -> bool {
    path.components().all(|component| match component {
        std::path::Component::Normal(name) => name
            .to_str()
            .map(|name| !name.starts_with('.'))
            .unwrap_or(false),
        _ => true,
    })
}

#[cfg(test)]
mod tests {
    use super::is_public_memory_path;
    use std::path::Path;

    #[test]
    fn public_memory_paths_hide_dot_components() {
        assert!(is_public_memory_path(Path::new("raw_memories.md")));
        assert!(is_public_memory_path(Path::new(
            "rollout_summaries/session.md"
        )));
        assert!(!is_public_memory_path(Path::new(".index/cache.json")));
        assert!(!is_public_memory_path(Path::new(
            "rollout_summaries/.draft.md"
        )));
    }
}
