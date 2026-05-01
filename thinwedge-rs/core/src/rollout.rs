use crate::config::Config;
pub use thinwedge_rollout::ARCHIVED_SESSIONS_SUBDIR;
pub use thinwedge_rollout::Cursor;
pub use thinwedge_rollout::EventPersistenceMode;
pub use thinwedge_rollout::INTERACTIVE_SESSION_SOURCES;
pub use thinwedge_rollout::RolloutRecorder;
pub use thinwedge_rollout::RolloutRecorderParams;
pub use thinwedge_rollout::SESSIONS_SUBDIR;
pub use thinwedge_rollout::SessionMeta;
pub use thinwedge_rollout::SortDirection;
pub use thinwedge_rollout::ThreadItem;
pub use thinwedge_rollout::ThreadSortKey;
pub use thinwedge_rollout::ThreadsPage;
pub use thinwedge_rollout::append_thread_name;
pub use thinwedge_rollout::find_archived_thread_path_by_id_str;
#[deprecated(note = "use find_thread_path_by_id_str")]
pub use thinwedge_rollout::find_conversation_path_by_id_str;
pub use thinwedge_rollout::find_thread_meta_by_name_str;
pub use thinwedge_rollout::find_thread_name_by_id;
pub use thinwedge_rollout::find_thread_names_by_ids;
pub use thinwedge_rollout::find_thread_path_by_id_str;
pub use thinwedge_rollout::parse_cursor;
pub use thinwedge_rollout::read_head_for_summary;
pub use thinwedge_rollout::read_session_meta_line;
pub use thinwedge_rollout::rollout_date_parts;

impl thinwedge_rollout::RolloutConfigView for Config {
    fn thinwedge_home(&self) -> &std::path::Path {
        self.thinwedge_home.as_path()
    }

    fn sqlite_home(&self) -> &std::path::Path {
        self.sqlite_home.as_path()
    }

    fn cwd(&self) -> &std::path::Path {
        self.cwd.as_path()
    }

    fn model_provider_id(&self) -> &str {
        self.model_provider_id.as_str()
    }

    fn generate_memories(&self) -> bool {
        self.memories.generate_memories
    }
}

pub(crate) mod list {
    pub use thinwedge_rollout::find_thread_path_by_id_str;
}

pub(crate) mod recorder {
    pub use thinwedge_rollout::RolloutRecorder;
}

pub(crate) use crate::session_rollout_init_error::map_session_init_error;

pub(crate) mod truncation {
    pub(crate) use crate::thread_rollout_truncation::*;
}
