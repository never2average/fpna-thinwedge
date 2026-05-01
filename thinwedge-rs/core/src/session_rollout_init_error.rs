use std::io::ErrorKind;
use std::path::Path;

use crate::rollout::SESSIONS_SUBDIR;
use thinwedge_protocol::error::ThinWedgeErr;

pub(crate) fn map_session_init_error(err: &anyhow::Error, thinwedge_home: &Path) -> ThinWedgeErr {
    if let Some(mapped) = err
        .chain()
        .filter_map(|cause| cause.downcast_ref::<std::io::Error>())
        .find_map(|io_err| map_rollout_io_error(io_err, thinwedge_home))
    {
        return mapped;
    }

    ThinWedgeErr::Fatal(format!("Failed to initialize session: {err:#}"))
}

fn map_rollout_io_error(io_err: &std::io::Error, thinwedge_home: &Path) -> Option<ThinWedgeErr> {
    let sessions_dir = thinwedge_home.join(SESSIONS_SUBDIR);
    let hint = match io_err.kind() {
        ErrorKind::PermissionDenied => format!(
            "ThinWedge cannot access session files at {} (permission denied). If sessions were created using sudo, fix ownership: sudo chown -R $(whoami) {}",
            sessions_dir.display(),
            thinwedge_home.display()
        ),
        ErrorKind::NotFound => format!(
            "Session storage missing at {}. Create the directory or choose a different ThinWedge home.",
            sessions_dir.display()
        ),
        ErrorKind::AlreadyExists => format!(
            "Session storage path {} is blocked by an existing file. Remove or rename it so ThinWedge can create sessions.",
            sessions_dir.display()
        ),
        ErrorKind::InvalidData | ErrorKind::InvalidInput => format!(
            "Session data under {} looks corrupt or unreadable. Clearing the sessions directory may help (this will remove saved threads).",
            sessions_dir.display()
        ),
        ErrorKind::IsADirectory | ErrorKind::NotADirectory => format!(
            "Session storage path {} has an unexpected type. Ensure it is a directory ThinWedge can use for session files.",
            sessions_dir.display()
        ),
        _ => return None,
    };

    Some(ThinWedgeErr::Fatal(format!(
        "{hint} (underlying error: {io_err})"
    )))
}
