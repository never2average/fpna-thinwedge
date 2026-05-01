use std::collections::HashMap;
use std::path::PathBuf;
use thinwedge_app_server_protocol::AdditionalNetworkPermissions;
use thinwedge_app_server_protocol::FileUpdateChange;
use thinwedge_app_server_protocol::GrantedPermissionProfile;
use thinwedge_app_server_protocol::NetworkApprovalContext as AppServerNetworkApprovalContext;
use thinwedge_app_server_protocol::PatchChangeKind;
use thinwedge_protocol::protocol::FileChange;
use thinwedge_protocol::protocol::NetworkApprovalContext;
use thinwedge_protocol::protocol::NetworkApprovalProtocol;
use thinwedge_protocol::request_permissions::RequestPermissionProfile as CoreRequestPermissionProfile;

pub(crate) fn network_approval_context_to_core(
    value: AppServerNetworkApprovalContext,
) -> NetworkApprovalContext {
    NetworkApprovalContext {
        host: value.host,
        protocol: match value.protocol {
            thinwedge_app_server_protocol::NetworkApprovalProtocol::Http => {
                NetworkApprovalProtocol::Http
            }
            thinwedge_app_server_protocol::NetworkApprovalProtocol::Https => {
                NetworkApprovalProtocol::Https
            }
            thinwedge_app_server_protocol::NetworkApprovalProtocol::Socks5Tcp => {
                NetworkApprovalProtocol::Socks5Tcp
            }
            thinwedge_app_server_protocol::NetworkApprovalProtocol::Socks5Udp => {
                NetworkApprovalProtocol::Socks5Udp
            }
        },
    }
}

pub(crate) fn granted_permission_profile_from_request(
    value: CoreRequestPermissionProfile,
) -> GrantedPermissionProfile {
    GrantedPermissionProfile {
        network: value.network.map(|network| AdditionalNetworkPermissions {
            enabled: network.enabled,
        }),
        file_system: value.file_system.map(Into::into),
    }
}

pub(crate) fn file_update_changes_to_core(
    changes: Vec<FileUpdateChange>,
) -> HashMap<PathBuf, FileChange> {
    changes
        .into_iter()
        .map(|change| {
            let path = PathBuf::from(change.path);
            let file_change = match change.kind {
                PatchChangeKind::Add => FileChange::Add {
                    content: change.diff,
                },
                PatchChangeKind::Delete => FileChange::Delete {
                    content: change.diff,
                },
                PatchChangeKind::Update { move_path } => FileChange::Update {
                    unified_diff: change.diff,
                    move_path,
                },
            };
            (path, file_change)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::file_update_changes_to_core;
    use super::granted_permission_profile_from_request;
    use super::network_approval_context_to_core;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use thinwedge_app_server_protocol::FileUpdateChange;
    use thinwedge_app_server_protocol::PatchChangeKind;
    use thinwedge_protocol::models::FileSystemPermissions;
    use thinwedge_protocol::models::NetworkPermissions;
    use thinwedge_protocol::permissions::FileSystemAccessMode;
    use thinwedge_protocol::permissions::FileSystemPath;
    use thinwedge_protocol::permissions::FileSystemSandboxEntry;
    use thinwedge_protocol::permissions::FileSystemSpecialPath;
    use thinwedge_protocol::protocol::FileChange;
    use thinwedge_protocol::protocol::NetworkApprovalContext;
    use thinwedge_protocol::protocol::NetworkApprovalProtocol;
    use thinwedge_protocol::request_permissions::RequestPermissionProfile as CoreRequestPermissionProfile;
    use thinwedge_utils_absolute_path::AbsolutePathBuf;

    fn absolute_path(path: &str) -> AbsolutePathBuf {
        AbsolutePathBuf::try_from(PathBuf::from(path)).expect("path must be absolute")
    }

    #[test]
    fn converts_app_server_network_approval_context_to_core() {
        assert_eq!(
            network_approval_context_to_core(
                thinwedge_app_server_protocol::NetworkApprovalContext {
                    host: "example.com".to_string(),
                    protocol: thinwedge_app_server_protocol::NetworkApprovalProtocol::Socks5Tcp,
                }
            ),
            NetworkApprovalContext {
                host: "example.com".to_string(),
                protocol: NetworkApprovalProtocol::Socks5Tcp,
            }
        );
    }

    #[test]
    fn converts_file_update_changes_to_core() {
        assert_eq!(
            file_update_changes_to_core(vec![FileUpdateChange {
                path: "foo.txt".to_string(),
                kind: PatchChangeKind::Add,
                diff: "hello\n".to_string(),
            }]),
            HashMap::from([(
                PathBuf::from("foo.txt"),
                FileChange::Add {
                    content: "hello\n".to_string(),
                },
            )])
        );
    }

    #[test]
    fn converts_request_permissions_into_granted_permissions() {
        assert_eq!(
            granted_permission_profile_from_request(CoreRequestPermissionProfile {
                network: Some(NetworkPermissions {
                    enabled: Some(true),
                }),
                file_system: Some(FileSystemPermissions::from_read_write_roots(
                    Some(vec![absolute_path("/tmp/read-only")]),
                    Some(vec![absolute_path("/tmp/write")]),
                )),
            }),
            thinwedge_app_server_protocol::GrantedPermissionProfile {
                network: Some(
                    thinwedge_app_server_protocol::AdditionalNetworkPermissions {
                        enabled: Some(true),
                    }
                ),
                file_system: Some(
                    thinwedge_app_server_protocol::AdditionalFileSystemPermissions {
                        read: Some(vec![absolute_path("/tmp/read-only")]),
                        write: Some(vec![absolute_path("/tmp/write")]),
                        glob_scan_max_depth: None,
                        entries: Some(vec![
                            thinwedge_app_server_protocol::FileSystemSandboxEntry {
                                path: thinwedge_app_server_protocol::FileSystemPath::Path {
                                    path: absolute_path("/tmp/read-only"),
                                },
                                access: thinwedge_app_server_protocol::FileSystemAccessMode::Read,
                            },
                            thinwedge_app_server_protocol::FileSystemSandboxEntry {
                                path: thinwedge_app_server_protocol::FileSystemPath::Path {
                                    path: absolute_path("/tmp/write"),
                                },
                                access: thinwedge_app_server_protocol::FileSystemAccessMode::Write,
                            },
                        ]),
                    }
                ),
            }
        );
    }

    #[test]
    fn converts_request_permissions_into_canonical_granted_permissions() {
        assert_eq!(
            granted_permission_profile_from_request(CoreRequestPermissionProfile {
                file_system: Some(FileSystemPermissions {
                    entries: vec![FileSystemSandboxEntry {
                        path: FileSystemPath::Special {
                            value: FileSystemSpecialPath::Root,
                        },
                        access: FileSystemAccessMode::Write,
                    }],
                    glob_scan_max_depth: None,
                }),
                ..Default::default()
            }),
            thinwedge_app_server_protocol::GrantedPermissionProfile {
                network: None,
                file_system: Some(
                    thinwedge_app_server_protocol::AdditionalFileSystemPermissions {
                        read: None,
                        write: None,
                        glob_scan_max_depth: None,
                        entries: Some(vec![
                            thinwedge_app_server_protocol::FileSystemSandboxEntry {
                                path: thinwedge_app_server_protocol::FileSystemPath::Special {
                                    value:
                                        thinwedge_app_server_protocol::FileSystemSpecialPath::Root,
                                },
                                access: thinwedge_app_server_protocol::FileSystemAccessMode::Write,
                            },
                        ]),
                    }
                ),
            }
        );
    }
}
