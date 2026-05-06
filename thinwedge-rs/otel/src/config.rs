use std::collections::HashMap;
use std::path::PathBuf;

use thinwedge_utils_absolute_path::AbsolutePathBuf;

const STATSIG_OTLP_HTTP_ENDPOINT_ENV: &str = "THINWEDGE_STATSIG_OTLP_HTTP_ENDPOINT";
const STATSIG_API_KEY_ENV: &str = "THINWEDGE_STATSIG_API_KEY";
const STATSIG_API_KEY_HEADER: &str = "statsig-api-key";

pub(crate) fn resolve_exporter(exporter: &OtelExporter) -> OtelExporter {
    match exporter {
        OtelExporter::Statsig => statsig_exporter_from_env().unwrap_or(OtelExporter::None),
        _ => exporter.clone(),
    }
}

fn statsig_exporter_from_env() -> Option<OtelExporter> {
    let endpoint = std::env::var(STATSIG_OTLP_HTTP_ENDPOINT_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())?;
    let api_key = std::env::var(STATSIG_API_KEY_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())?;

    Some(OtelExporter::OtlpHttp {
        endpoint,
        headers: [(STATSIG_API_KEY_HEADER.to_string(), api_key)].into(),
        protocol: OtelHttpProtocol::Json,
        tls: None,
    })
}

#[derive(Clone, Debug)]
pub struct OtelSettings {
    pub environment: String,
    pub service_name: String,
    pub service_version: String,
    pub thinwedge_home: PathBuf,
    pub exporter: OtelExporter,
    pub trace_exporter: OtelExporter,
    pub metrics_exporter: OtelExporter,
    pub runtime_metrics: bool,
}

#[derive(Clone, Debug)]
pub enum OtelHttpProtocol {
    /// HTTP protocol with binary protobuf
    Binary,
    /// HTTP protocol with JSON payload
    Json,
}

#[derive(Clone, Debug, Default)]
pub struct OtelTlsConfig {
    pub ca_certificate: Option<AbsolutePathBuf>,
    pub client_certificate: Option<AbsolutePathBuf>,
    pub client_private_key: Option<AbsolutePathBuf>,
}

#[derive(Clone, Debug)]
pub enum OtelExporter {
    None,
    /// Statsig metrics ingestion exporter using ThinWedge-internal defaults.
    ///
    /// This is intended for metrics only.
    Statsig,
    OtlpGrpc {
        endpoint: String,
        headers: HashMap<String, String>,
        tls: Option<OtelTlsConfig>,
    },
    OtlpHttp {
        endpoint: String,
        headers: HashMap<String, String>,
        protocol: OtelHttpProtocol,
        tls: Option<OtelTlsConfig>,
    },
}

#[cfg(test)]
mod tests {
    use super::OtelExporter;
    use super::resolve_exporter;

    #[test]
    fn statsig_exporter_is_disabled_without_env_config() {
        assert!(matches!(
            resolve_exporter(&OtelExporter::Statsig),
            OtelExporter::None
        ));
    }
}
