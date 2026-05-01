pub mod installed_marketplaces;
pub mod loader;
pub mod manifest;
pub mod marketplace;
pub mod marketplace_add;
pub mod marketplace_remove;
pub mod marketplace_upgrade;
pub mod remote;
pub mod remote_bundle;
pub mod remote_legacy;
pub mod startup_sync;
pub mod store;
pub mod toggles;

pub const THINWEDGE_CURATED_MARKETPLACE_NAME: &str = "thinwedge-curated";
pub const THINWEDGE_BUNDLED_MARKETPLACE_NAME: &str = "thinwedge-bundled";
