// Perihelion - Game-agnostic native Linux mod manager and VFS orchestrator.
// Copyright (C) 2026 Eduardo Marques Santos and Perihelion Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Perihelion VFS Adapter (`peri-vfs`)
//!
//! Handles Linux mount namespaces and OverlayFS stack orchestration to deliver
//! zero-overhead, non-destructive game sandboxing.

pub mod engine;
pub mod proton;

pub use engine::{
    detect_engine_layout, find_executable_for_engine, probe_game_directory, wildcard_match,
};
pub use proton::{DiscoveredSteamApp, ProtonLaunch, ProtonTool, SteamDiscovery};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use nix::mount::{mount, umount2, MntFlags, MsFlags};
use nix::sched::{unshare, CloneFlags};
use nix::unistd::{getegid, geteuid};
use thiserror::Error;
use tracing::{debug, info, info_span, warn};

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

    let span = info_span!("mount_overlay", mount_point = %plan.mount_point.display());
    let _enter = span.enter();
    debug!(options = %options, "Mounting OverlayFS");

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
    debug!(mount_point = %mount_point.display(), "Detaching OverlayFS mount");
    umount2(mount_point, MntFlags::MNT_DETACH)?;
    Ok(())
}

/// Execution sandbox policy for confining untrusted game binaries and mod scripts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxPolicy {
    /// Remount the host root filesystem ('/') as read-only inside the namespace.
    pub readonly_root: bool,
    /// Explicit whitelist of paths that retain read-write access inside the namespace.
    pub writable_paths: Vec<PathBuf>,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            readonly_root: true,
            writable_paths: vec![PathBuf::from("/tmp"), PathBuf::from("/dev/shm")],
        }
    }
}

impl SandboxPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_readonly_root(mut self, enabled: bool) -> Self {
        self.readonly_root = enabled;
        self
    }

    pub fn with_writable_path(mut self, path: PathBuf) -> Self {
        if !self.writable_paths.contains(&path) {
            self.writable_paths.push(path);
        }
        self
    }

    pub fn with_writable_paths(mut self, paths: impl IntoIterator<Item = PathBuf>) -> Self {
        for p in paths {
            self = self.with_writable_path(p);
        }
        self
    }
}

/// Applies Phase B execution boundary hardening:
/// 1. Makes all mount events private to this isolated namespace.
/// 2. Remounts the entire root filesystem as read-only.
/// 3. Whitelists only specific directories (upperdir, workdir, compatdata, /tmp) as writable.
fn apply_sandbox_policy(plan: &VfsPlan, policy: &SandboxPolicy) -> Result<(), VfsError> {
    // 1. Prevent mount events from propagating to or from the parent namespace
    let _ = mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_REC | MsFlags::MS_PRIVATE,
        None::<&str>,
    );

    if policy.readonly_root {
        info!("Enforcing zero-trust execution boundary: remounting '/' read-only");

        // Collect all explicitly whitelisted writable paths
        let mut writable = policy.writable_paths.clone();
        if let Some(upper) = &plan.upper_dir {
            writable.push(upper.clone());
        }
        if let Some(work) = &plan.work_dir {
            writable.push(work.clone());
        }

        debug!(
            writable_count = writable.len(),
            "Configuring sandbox writable paths"
        );

        // Bind mount each writable path onto itself so it becomes an independent mount point
        for path in &writable {
            if path.exists() {
                let _ = mount(
                    Some(path.as_path()),
                    path.as_path(),
                    None::<&str>,
                    MsFlags::MS_BIND | MsFlags::MS_REC,
                    None::<&str>,
                );
            }
        }

        // Bind mount / onto itself to establish an owned root mount inside our namespace
        let _ = mount(
            Some("/"),
            "/",
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_REC,
            None::<&str>,
        );

        // Remount root filesystem as read-only
        if let Err(e) = mount(
            Some("none"),
            "/",
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_REMOUNT | MsFlags::MS_RDONLY | MsFlags::MS_REC,
            None::<&str>,
        ) {
            warn!(
                error = %e,
                "Read-only root remount skipped (restricted by host/container environment); continuing with namespace sandboxing"
            );
        }

        // Re-ensure whitelisted writable paths maintain read-write flags
        for path in &writable {
            if path.exists() {
                let _ = mount(
                    Some("none"),
                    path.as_path(),
                    None::<&str>,
                    MsFlags::MS_BIND | MsFlags::MS_REMOUNT | MsFlags::MS_REC,
                    None::<&str>,
                );
            }
        }
    }

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
    execute_in_isolated_namespace_with_policy(
        plan,
        &SandboxPolicy::default(),
        command,
        args,
        env_vars,
    )
}

/// Spawns an isolated Linux user + mount namespace with a custom sandbox policy and environment variables.
pub fn execute_in_isolated_namespace_with_policy(
    plan: &VfsPlan,
    policy: &SandboxPolicy,
    command: &str,
    args: &[String],
    env_vars: &[(String, String)],
) -> Result<i32, VfsError> {
    let span = info_span!("execute_in_isolated_namespace", command = %command);
    let _enter = span.enter();
    debug!("Spawning isolated namespace and configuring sandbox");

    let euid = geteuid();
    let egid = getegid();

    // 1. Spawn new user and mount namespaces
    unshare(CloneFlags::CLONE_NEWUSER | CloneFlags::CLONE_NEWNS)
        .map_err(|e| VfsError::NamespaceFailed(format!("unshare failed: {e}")))?;

    // 2. Map UID and GID to allow unprivileged root inside the namespace
    setup_id_mapping(euid.as_raw(), egid.as_raw())?;

    // 3. Apply Zero-Trust execution boundary (read-only root + writable whitelisting)
    apply_sandbox_policy(plan, policy)?;

    // 4. Mount OverlayFS within the isolated namespace
    mount_overlay(plan)?;

    // 5. Execute target process
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
/// Automatically whitelists the Proton compatdata and Wine prefix directories as writable.
pub fn execute_proton_in_isolated_namespace(
    plan: &VfsPlan,
    proton: &ProtonLaunch,
    game_executable: &str,
    args: &[String],
) -> Result<i32, VfsError> {
    let policy = SandboxPolicy::default()
        .with_writable_path(proton.compat_data_path.clone())
        .with_writable_path(proton.wine_prefix.clone());

    execute_proton_in_isolated_namespace_with_policy(plan, &policy, proton, game_executable, args)
}

/// Executes a Windows game binary via Proton with an explicit sandbox policy.
pub fn execute_proton_in_isolated_namespace_with_policy(
    plan: &VfsPlan,
    policy: &SandboxPolicy,
    proton: &ProtonLaunch,
    game_executable: &str,
    args: &[String],
) -> Result<i32, VfsError> {
    let mut proton_args = vec!["run".to_string(), game_executable.to_string()];
    proton_args.extend(args.iter().cloned());

    execute_in_isolated_namespace_with_policy(
        plan,
        policy,
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

    #[test]
    fn test_sandbox_policy_configuration() {
        let policy = SandboxPolicy::default()
            .with_readonly_root(true)
            .with_writable_path(PathBuf::from("/steamapps/compatdata/489830"))
            .with_writable_paths(vec![PathBuf::from("/tmp/custom")]);

        assert!(policy.readonly_root);
        assert!(policy
            .writable_paths
            .contains(&PathBuf::from("/steamapps/compatdata/489830")));
        assert!(policy
            .writable_paths
            .contains(&PathBuf::from("/tmp/custom")));
        assert!(policy.writable_paths.contains(&PathBuf::from("/tmp")));
    }
}
