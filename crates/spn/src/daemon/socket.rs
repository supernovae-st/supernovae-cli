//! Socket utilities for secure IPC.
//!
//! Provides SO_PEERCRED verification and socket permission management.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::Path;
use tracing::{debug, warn};

use super::DaemonError;

/// Socket utilities for daemon communication.
pub struct SocketUtils;

impl SocketUtils {
    /// Set socket file permissions to 0600 (owner only).
    pub fn set_socket_permissions(path: &Path) -> Result<(), DaemonError> {
        let permissions = fs::Permissions::from_mode(0o600);
        fs::set_permissions(path, permissions).map_err(|source| {
            DaemonError::SetPermissionsFailed {
                path: path.to_path_buf(),
                source,
            }
        })
    }

    /// Verify socket file ownership.
    ///
    /// Ensures the socket is owned by the current user.
    pub fn verify_socket_ownership(path: &Path) -> Result<(), DaemonError> {
        let metadata = fs::metadata(path).map_err(DaemonError::IoError)?;

        let file_uid = std::os::unix::fs::MetadataExt::uid(&metadata);
        let current_uid = unsafe { libc::getuid() };

        if file_uid != current_uid {
            return Err(DaemonError::PeerCredentialsFailed {
                reason: format!(
                    "Socket owned by UID {} but current UID is {}",
                    file_uid, current_uid
                ),
            });
        }

        Ok(())
    }

    /// Remove stale socket file if it exists.
    pub fn cleanup_stale_socket(path: &Path) -> Result<(), DaemonError> {
        if path.exists() {
            warn!("Removing stale socket file: {:?}", path);
            fs::remove_file(path).map_err(DaemonError::IoError)?;
        }
        Ok(())
    }
}

/// Verify peer credentials on a Unix socket connection.
///
/// Uses SO_PEERCRED to get the UID of the connecting process and
/// verifies it matches the daemon's UID.
#[cfg(target_os = "linux")]
pub fn verify_peer_credentials(stream: &StdUnixStream) -> Result<PeerCredentials, DaemonError> {
    use std::os::unix::io::AsRawFd;

    let fd = stream.as_raw_fd();
    let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;

    let result = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            &mut cred as *mut _ as *mut libc::c_void,
            &mut len,
        )
    };

    if result != 0 {
        return Err(DaemonError::PeerCredentialsFailed {
            reason: format!("getsockopt failed: {}", std::io::Error::last_os_error()),
        });
    }

    let peer = PeerCredentials {
        pid: cred.pid as u32,
        uid: cred.uid,
        gid: cred.gid,
    };

    // Verify UID matches
    let daemon_uid = unsafe { libc::getuid() };
    if peer.uid != daemon_uid {
        return Err(DaemonError::Unauthorized {
            peer_uid: peer.uid,
            daemon_uid,
        });
    }

    debug!("Verified peer: PID={}, UID={}, GID={}", peer.pid, peer.uid, peer.gid);
    Ok(peer)
}

/// Verify peer credentials on macOS.
///
/// macOS uses LOCAL_PEERCRED instead of SO_PEERCRED.
#[cfg(target_os = "macos")]
pub fn verify_peer_credentials(stream: &StdUnixStream) -> Result<PeerCredentials, DaemonError> {
    use std::os::unix::io::AsRawFd;

    let fd = stream.as_raw_fd();

    // macOS xucred structure
    #[repr(C)]
    struct xucred {
        cr_version: u32,
        cr_uid: libc::uid_t,
        cr_ngroups: libc::c_short,
        cr_groups: [libc::gid_t; 16],
    }

    let mut cred: xucred = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<xucred>() as libc::socklen_t;

    // LOCAL_PEERCRED = 0x001
    const LOCAL_PEERCRED: libc::c_int = 0x001;
    // SOL_LOCAL = 0 on macOS
    const SOL_LOCAL: libc::c_int = 0;

    let result = unsafe {
        libc::getsockopt(
            fd,
            SOL_LOCAL,
            LOCAL_PEERCRED,
            &mut cred as *mut _ as *mut libc::c_void,
            &mut len,
        )
    };

    if result != 0 {
        return Err(DaemonError::PeerCredentialsFailed {
            reason: format!("getsockopt failed: {}", std::io::Error::last_os_error()),
        });
    }

    let peer = PeerCredentials {
        pid: 0, // macOS LOCAL_PEERCRED doesn't provide PID
        uid: cred.cr_uid,
        gid: if cred.cr_ngroups > 0 {
            cred.cr_groups[0]
        } else {
            0
        },
    };

    // Verify UID matches
    let daemon_uid = unsafe { libc::getuid() };
    if peer.uid != daemon_uid {
        return Err(DaemonError::Unauthorized {
            peer_uid: peer.uid,
            daemon_uid,
        });
    }

    debug!("Verified peer: UID={}, GID={}", peer.uid, peer.gid);
    Ok(peer)
}

/// Peer credentials from SO_PEERCRED/LOCAL_PEERCRED.
#[derive(Debug, Clone)]
pub struct PeerCredentials {
    /// Process ID (0 on macOS)
    pub pid: u32,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_set_socket_permissions() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.sock");

        // Create a file to test permissions
        fs::write(&path, "").unwrap();

        SocketUtils::set_socket_permissions(&path).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn test_cleanup_stale_socket() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("stale.sock");

        // Create a file
        fs::write(&path, "").unwrap();
        assert!(path.exists());

        // Clean it up
        SocketUtils::cleanup_stale_socket(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_cleanup_nonexistent_socket() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.sock");

        // Should not error
        SocketUtils::cleanup_stale_socket(&path).unwrap();
    }
}
