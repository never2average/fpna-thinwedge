mod cost_context;
mod execution;
mod spec;
mod storage;
mod types;

pub(crate) use spec::dynamic_tool_specs;
pub(crate) use spec::handle_dynamic_tool_call;

const DEFAULT_ROLE_NAME: &str = "CFO";
const THINWEDGE_DATA_DIR: &str = "thinwedge/ml";
const MODELS_FILE_NAME: &str = "statisticalmodels.json";
const ENVIRONMENTS_FILE_NAME: &str = "trainingenvironments.json";
const JOBS_DIR_NAME: &str = "jobs";
const EVALS_DIR_NAME: &str = "evals";

#[cfg(test)]
mod tests;
