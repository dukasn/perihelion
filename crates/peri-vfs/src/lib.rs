//! Perihelion VFS Adapter (`peri-vfs`)
//!
//! Handles Linux mount namespaces and OverlayFS stack orchestration to deliver
//! zero-overhead, non-destructive game sandboxing.

pub mod proton;

pub use proton::{ProtonLaunch, ProtonTool, SteamDiscovery};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use nix::mount::{mount, umount2, MntFlags, MsFlags};
use nix::sched::{unshare, CloneFlags};
use nix::unistd::{getegid, geteuid};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VfsError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Nix system call error: {0}")]
    Nix(#[from] nix::Error),

    #[error("Invalid VFS configuration: {0}")]
    InvalidConfig(String),

    #[error("Mount failure on '{mount_point}': {reason}")]
    MountFailed {
        mount_point: PathBuf,
        reason: String,
    },

    #[error("Namespace configuration failure: {0}")]
    NamespaceFailed(String),

    #[error("Proton / Wine resolution error: {0}")]
    ProtonDiscoveryFailed(String),

    #[error("Execution failure in sandbox: {0}")]
    ExecutionFailed(String),
}

/// Represents an OverlayFS configuration plan derived from game profiles and the mod DAG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VfsPlan {
    /// Base game directory (bottom-most lower directory)
    pub base_dir: PathBuf,
    /// Mod directories ordered from lowest precedence to highest precedence
    pub layers: Vec<PathBuf>,
    /// Optional writable upper directory for modifications/saves
    pub upper_dir: Option<PathBuf>,
    /// Working directory required by OverlayFS if upper_dir is specified
    pub work_dir: Option<PathBuf>,
    /// Target directory where the merged VFS is exposed
    pub mount_point: PathBuf,
}

impl VfsPlan {
    pub fn new(base_dir: PathBuf, mount_point: PathBuf) -> Self {
        Self {
            base_dir,
            layers: Vec::new(),
            upper_dir: None,
            work_dir: None,
            mount_point,
        }
    }

    pub fn with_layers(mut self, layers: Vec<PathBuf>) -> Self {
        self.layers = layers;
        self
    }

    pub fn with_upper(mut self, upper_dir: PathBuf, work_dir: PathBuf) -> Self {
        self.upper_dir = Some(upper_dir);
        self.work_dir = Some(work_dir);
        self
    }

    /// Formats the OverlayFS mount options string.
    ///
    /// OverlayFS lowerdir takes precedence from left to right:
    /// `lowerdir=highest_precedence:...:lowest_precedence:base_game`
    pub fn format_mount_options(&self) -> Result<String, VfsError> {
        let mut lower_paths: Vec<String> = Vec::new();

        // Reverse layers so highest precedence (latest loaded mod) appears first in lowerdir
        for layer in self.layers.iter().rev() {
            let canon = layer
                .canonicalize()
                .unwrap_or_else(|_| layer.clone())
                .to_string_lossy()
                .into_owned();
            lower_paths.push(canon);
        }

        let base_canon = self
            .base_dir
            .canonicalize()
            .unwrap_or_else(|_| self.base_dir.clone())
            .to_string_lossy()
            .into_owned();
        lower_paths.push(base_canon);

        let lowerdir_opt = format!("lowerdir={}", lower_paths.join(":"));

        match (&self.upper_dir, &self.work_dir) {
            (Some(upper), Some(work)) => {
                let upper_str = upper
                    .canonicalize()
                    .unwrap_or_else(|_| upper.clone())
                    .to_string_lossy()
                    .into_owned();
                let work_str = work
                    .canonicalize()
                    .unwrap_or_else(|_| work.clone())
                    .to_string_lossy()
                    .into_owned();
                Ok(format!(
                    "{lowerdir_opt},upperdir={upper_str},workdir={work_str}"
                ))
            }
            (None, None) => Ok(lowerdir_opt),
            _ => Err(VfsError::InvalidConfig(
                "Both upperdir and workdir must be set if a writable layer is desired".to_string(),
            )),
        }
    }
}

/// Mounts an OverlayFS stack according to the provided VfsPlan.
pub fn mount_overlay(plan: &VfsPlan) -> Result<(), VfsError> {
    fs::create_dir_all(&plan.mount_point)?;

    if let Some(upper) = &plan.upper_dir {
        fs::create_dir_all(upper)?;
    }
    if let Some(work) = &plan.work_dir {
        fs::create_dir_all(work)?;
    }

    let options = plan.format_mount_options()?;

    mount(
        Some("overlay"),
        &plan.mount_point,
        Some("overlay"),
        MsFlags::empty(),
        Some(options.as_str()),
    )
    .map_err(|e| VfsError::MountFailed {
        mount_point: plan.mount_point.clone(),
        reason: format!("System mount call failed: {e}. (Options: {options})"),
    })?;

    Ok(())
}

/// Unmounts an existing overlay mount point.
pub fn unmount_overlay(mount_point: &Path) -> Result<(), VfsError> {
    umount2(mount_point, MntFlags::MNT_DETACH)?;
    Ok(())
}

/// Spawns an isolated Linux user + mount namespace and executes a target command within it.
pub fn execute_in_isolated_namespace(
    plan: &VfsPlan,
    command: &str,
    args: &[String],
) -> Result<i32, VfsError> {
    execute_in_isolated_namespace_with_env(plan, command, args, &[])
}

/// Spawns an isolated Linux user + mount namespace with custom environment variables.
pub fn execute_in_isolated_namespace_with_env(
    plan: &VfsPlan,
    command: &str,
    args: &[String],
    env_vars: &[(String, String)],
) -> Result<i32, VfsError> {
    let euid = geteuid();
    let egid = getegid();

    // Spawn new user and mount namespaces
    unshare(CloneFlags::CLONE_NEWUSER | CloneFlags::CLONE_NEWNS)
        .map_err(|e| VfsError::NamespaceFailed(format!("unshare failed: {e}")))?;

    // Map UID and GID to allow unprivileged root inside the namespace
    setup_id_mapping(euid.as_raw(), egid.as_raw())?;

    // Mount OverlayFS within the isolated namespace
    mount_overlay(plan)?;

    // Execute target process
    let mut cmd = Command::new(command);
    cmd.args(args).current_dir(&plan.mount_point);

    for (k, v) in env_vars {
        cmd.env(k, v);
    }

    let status = cmd
        .status()
        .map_err(|e| VfsError::ExecutionFailed(format!("Failed to execute process: {e}")))?;

    Ok(status.code().unwrap_or(-1))
}

/// Executes a Windows game binary via Proton inside the sandboxed OverlayFS namespace.
pub fn execute_proton_in_isolated_namespace(
    plan: &VfsPlan,
    proton: &ProtonLaunch,
    game_executable: &str,
    args: &[String],
) -> Result<i32, VfsError> {
    let mut proton_args = vec!["run".to_string(), game_executable.to_string()];
    proton_args.extend(args.iter().cloned());

    execute_in_isolated_namespace_with_env(
        plan,
        &proton.runner.to_string_lossy(),
        &proton_args,
        &proton.env_vars,
    )
}

fn setup_id_mapping(uid: u32, gid: u32) -> Result<(), VfsError> {
    if Path::new("/proc/self/setgroups").exists() {
        let _ = fs::write("/proc/self/setgroups", "deny");
    }

    fs::write("/proc/self/uid_map", format!("0 {uid} 1"))
        .map_err(|e| VfsError::NamespaceFailed(format!("Failed to write uid_map: {e}")))?;

    fs::write("/proc/self/gid_map", format!("0 {gid} 1"))
        .map_err(|e| VfsError::NamespaceFailed(format!("Failed to write gid_map: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vfs_plan_lowerdir_ordering() {
        let plan = VfsPlan::new(
            PathBuf::from("/games/cyberpunk"),
            PathBuf::from("/mnt/target"),
        )
        .with_layers(vec![
            PathBuf::from("/mods/core_engine"),
            PathBuf::from("/mods/ui_mod"),
        ]);

        let opts = plan.format_mount_options().unwrap();
        // Highest precedence (ui_mod) must come before lower precedence (core_engine)
        // and base game must be at the very end
        assert!(opts.starts_with("lowerdir="));
        let lowerdirs = opts.strip_prefix("lowerdir=").unwrap();
        let parts: Vec<&str> = lowerdirs.split(':').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts[0].ends_with("ui_mod"));
        assert!(parts[1].ends_with("core_engine"));
        assert!(parts[2].ends_with("cyberpunk"));
    }

    #[test]
    fn test_vfs_plan_with_upper() {
        let plan = VfsPlan::new(PathBuf::from("/games/base"), PathBuf::from("/mnt/target"))
            .with_upper(PathBuf::from("/tmp/upper"), PathBuf::from("/tmp/work"));

        let opts = plan.format_mount_options().unwrap();
        assert!(opts.contains("upperdir="));
        assert!(opts.contains("workdir="));
    }
}
