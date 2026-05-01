use env_flags::env_flags;

env_flags! {
    /// Fixture path for offline tests (see client.rs).
    pub THINWEDGE_RS_SSE_FIXTURE: Option<&str> = None;
}
