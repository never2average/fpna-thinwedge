pub(crate) use thinwedge_skills::install_system_skills;
pub(crate) use thinwedge_skills::system_cache_root_dir;

use thinwedge_utils_absolute_path::AbsolutePathBuf;

pub(crate) fn uninstall_system_skills(thinwedge_home: &AbsolutePathBuf) {
    let _ = std::fs::remove_dir_all(system_cache_root_dir(thinwedge_home));
}
