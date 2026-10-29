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

pub mod base_game;
pub mod engine;
pub mod proton;
pub mod runner;

pub use base_game::{resolve_base_game_path, BaseGameQuery, BaseGameSource, ResolvedBaseGame};
pub use engine::{
    detect_engine_layout, find_executable_for_engine, probe_game_directory, wildcard_match,
};
pub use proton::{DiscoveredSteamApp, ProtonLaunch, ProtonTool, SteamDiscovery};
pub use runner::{CommandWrapper, RunnerKind, RunnerPipeline};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use nix::mount::{mount, umount2, MntFlags, MsFlags};
use nix::sched::{unshare, CloneFlags};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{fork, getegid, geteuid, ForkResult};
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

    #[error("Base game resolution failed: {0}")]
    BaseGameNotFound(String),
}

/// Represents an OverlayFS configuration plan derived from game profiles and the mod DAG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VfsPlan {
    /// Base game directory (bottom-most lower directory)
    pub base_dir: PathBuf,
    /// Arbitrary lower root directories (bottom-most lower directories)
    pub lower_roots: Vec<PathBuf>,
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
            base_dir: base_dir.clone(),
            lower_roots: vec![base_dir],
            layers: Vec::new(),
            upper_dir: None,
            work_dir: None,
            mount_point,
        }
    }

    pub fn from_roots(lower_roots: Vec<PathBuf>, mount_point: PathBuf) -> Result<Self, VfsError> {
        if lower_roots.is_empty() {
            return Err(VfsError::InvalidConfig(
                "At least one lower root must be specified".into(),
            ));
        }
        let base_dir = lower_roots.first().unwrap().clone();
        Ok(Self {
            base_dir,
            lower_roots,
            layers: Vec::new(),
            upper_dir: None,
            work_dir: None,
            mount_point,
        })
    }

    pub fn with_layers(mut self, layers: Vec<PathBuf>) -> Self {
        self.layers = layers;
        self
    }

    pub fn with_lower_root(mut self, root: PathBuf) -> Self {
        if !self.lower_roots.contains(&root) {
            self.lower_roots.push(root);
        }
        self
    }

    pub fn with_lower_roots(mut self, roots: Vec<PathBuf>) -> Self {
        if let Some(first) = roots.first() {
            self.base_dir = first.clone();
        }
        self.lower_roots = roots;
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
    /// `lowerdir=highest_precedence:...:lowest_precedence:base_root1:...:base_rootN`
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

        let roots = if self.lower_roots.is_empty() {
            vec![self.base_dir.clone()]
        } else {
            self.lower_roots.clone()
        };

        for root in roots {
            let canon = root
                .canonicalize()
                .unwrap_or_else(|_| root.clone())
                .to_string_lossy()
                .into_owned();
            lower_paths.push(canon);
        }

        if lower_paths.is_empty() {
            return Err(VfsError::InvalidConfig(
                "At least one lowerdir layer or base root must be provided for OverlayFS".into(),
            ));
        }

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

/// Mounts that must stay writable for the process to function (devices, procfs, runtime sockets).
const READONLY_EXEMPT_PREFIXES: [&str; 4] = ["/proc", "/sys", "/dev", "/run"];

/// Parses `/proc/self/mountinfo` into `(mount point, per-mount flags, already read-only)`.
fn parse_mountinfo(content: &str) -> Vec<(PathBuf, MsFlags, bool)> {
    content
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let mount_point = unescape_mountinfo(fields.nth(4)?);
            let options = fields.next()?;

            let mut flags = MsFlags::empty();
            let mut readonly = false;
            for opt in options.split(',') {
                match opt {
                    "ro" => readonly = true,
                    "nosuid" => flags |= MsFlags::MS_NOSUID,
                    "nodev" => flags |= MsFlags::MS_NODEV,
                    "noexec" => flags |= MsFlags::MS_NOEXEC,
                    "noatime" => flags |= MsFlags::MS_NOATIME,
                    "nodiratime" => flags |= MsFlags::MS_NODIRATIME,
                    "relatime" => flags |= MsFlags::MS_RELATIME,
                    _ => {}
                }
            }
            Some((PathBuf::from(mount_point), flags, readonly))
        })
        .collect()
}

/// Decodes the `\040`-style octal escapes mountinfo uses for spaces and tabs in paths.
fn unescape_mountinfo(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\'
            && i + 3 < bytes.len()
            && bytes[i + 1..i + 4].iter().all(u8::is_ascii_digit)
        {
            let value =
                (bytes[i + 1] - b'0') * 64 + (bytes[i + 2] - b'0') * 8 + (bytes[i + 3] - b'0');
            out.push(value);
            i += 4;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Remounts every mount in the namespace read-only except exempt system mounts and `writable` trees.
fn remount_all_readonly(writable: &[PathBuf]) {
    let Ok(content) = fs::read_to_string("/proc/self/mountinfo") else {
        warn!("Cannot read /proc/self/mountinfo; read-only root not enforced");
        return;
    };

    let mut failed = 0usize;
    for (path, flags, readonly) in parse_mountinfo(&content) {
        let exempt = READONLY_EXEMPT_PREFIXES.iter().any(|p| path.starts_with(p))
            || writable.iter().any(|w| path.starts_with(w));
        if readonly || exempt {
            continue;
        }

        // Locked mount flags must be re-specified or the kernel rejects the remount.
        let result = mount(
            None::<&str>,
            path.as_path(),
            None::<&str>,
            MsFlags::MS_REMOUNT | MsFlags::MS_BIND | MsFlags::MS_RDONLY | flags,
            None::<&str>,
        );
        if let Err(e) = result {
            failed += 1;
            debug!(mount = %path.display(), error = %e, "Could not remount read-only");
        }
    }

    if failed > 0 {
        warn!(
            failed,
            "Some mounts could not be made read-only (restricted by host environment)"
        );
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
        // overlayfs rejects upperdir/workdir on different mounts (EINVAL), so
        // they must share one bind mount via their common ancestor.
        match (&plan.upper_dir, &plan.work_dir) {
            (Some(upper), Some(work)) => {
                let common: PathBuf = upper
                    .components()
                    .zip(work.components())
                    .take_while(|(a, b)| a == b)
                    .map(|(a, _)| a)
                    .collect();
                writable.push(common);
            }
            (Some(dir), None) | (None, Some(dir)) => writable.push(dir.clone()),
            (None, None) => {}
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

        // MS_REC is ignored on remount, so every mount must be made read-only one by one.
        remount_all_readonly(&writable);

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

/// Executes an arbitrary `RunnerPipeline` inside an isolated Linux user + mount namespace.
/// Automatically constructs a hardened `SandboxPolicy` whitelisting runner prefixes and runtime paths.
pub fn execute_pipeline_in_isolated_namespace(
    plan: &VfsPlan,
    pipeline: &RunnerPipeline,
) -> Result<i32, VfsError> {
    let policy = pipeline.build_sandbox_policy(plan);
    execute_pipeline_in_isolated_namespace_with_policy(plan, &policy, pipeline)
}

/// Executes a `RunnerPipeline` inside an isolated namespace with an explicit `SandboxPolicy`.
pub fn execute_pipeline_in_isolated_namespace_with_policy(
    plan: &VfsPlan,
    policy: &SandboxPolicy,
    pipeline: &RunnerPipeline,
) -> Result<i32, VfsError> {
    let span = info_span!("execute_pipeline", runner = pipeline.runner_name());
    let _enter = span.enter();
    debug!("Spawning isolated namespace and executing runner pipeline");

    // CLONE_NEWUSER fails with EINVAL in a multi-threaded process (tokio runtime),
    // so the namespace work happens in a forked single-threaded child.
    // SAFETY: the child only runs syscalls and process spawning, then _exit()s.
    match unsafe { fork() }? {
        ForkResult::Child => {
            let code = match run_in_new_namespaces(plan, policy, pipeline) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("{e}");
                    CHILD_SETUP_FAILED
                }
            };
            // SAFETY: _exit skips destructors/atexit handlers inherited from the parent.
            unsafe { nix::libc::_exit(code) }
        }
        ForkResult::Parent { child } => match waitpid(child, None)? {
            WaitStatus::Exited(_, CHILD_SETUP_FAILED) => Err(VfsError::NamespaceFailed(
                "sandbox setup failed (details printed above)".to_string(),
            )),
            WaitStatus::Exited(_, code) => Ok(code),
            WaitStatus::Signaled(_, sig, _) => Ok(128 + sig as i32),
            other => Err(VfsError::ExecutionFailed(format!(
                "unexpected child status: {other:?}"
            ))),
        },
    }
}

const CHILD_SETUP_FAILED: i32 = 125;

fn run_in_new_namespaces(
    plan: &VfsPlan,
    policy: &SandboxPolicy,
    pipeline: &RunnerPipeline,
) -> Result<i32, VfsError> {
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

    // 5. Build and execute target command
    let mut cmd = pipeline.build_command(&plan.mount_point)?;
    let status = cmd
        .status()
        .map_err(|e| VfsError::ExecutionFailed(format!("Failed to execute process: {e}")))?;

    Ok(status.code().unwrap_or(-1))
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
    let pipeline = RunnerPipeline::native(command)
        .with_args(args.iter().cloned())
        .with_envs(env_vars.iter().cloned());
    execute_pipeline_in_isolated_namespace_with_policy(plan, policy, &pipeline)
}

/// Executes a Windows game binary via Proton inside the sandboxed OverlayFS namespace.
/// Automatically whitelists the Proton compatdata and Wine prefix directories as writable.
pub fn execute_proton_in_isolated_namespace(
    plan: &VfsPlan,
    proton: &ProtonLaunch,
    game_executable: &str,
    args: &[String],
) -> Result<i32, VfsError> {
    let pipeline =
        RunnerPipeline::proton(proton.clone(), game_executable).with_args(args.iter().cloned());
    execute_pipeline_in_isolated_namespace(plan, &pipeline)
}

/// Executes a Windows game binary via Proton with an explicit sandbox policy.
pub fn execute_proton_in_isolated_namespace_with_policy(
    plan: &VfsPlan,
    policy: &SandboxPolicy,
    proton: &ProtonLaunch,
    game_executable: &str,
    args: &[String],
) -> Result<i32, VfsError> {
    let pipeline =
        RunnerPipeline::proton(proton.clone(), game_executable).with_args(args.iter().cloned());
    execute_pipeline_in_isolated_namespace_with_policy(plan, policy, &pipeline)
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
    fn test_parse_mountinfo_flags_and_escapes() {
        let info = "36 35 0:36 /home /var/home/my\\040games rw,nosuid,nodev,relatime shared:1 - btrfs /dev/x rw\n\
                    40 35 0:40 / /proc ro,nosuid - proc proc rw\n";
        let mounts = parse_mountinfo(info);
        assert_eq!(mounts.len(), 2);
        assert_eq!(mounts[0].0, PathBuf::from("/var/home/my games"));
        assert_eq!(
            mounts[0].1,
            MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_RELATIME
        );
        assert!(!mounts[0].2);
        assert!(mounts[1].2);
    }

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
    fn test_vfs_plan_arbitrary_absolute_lower_roots() {
        let plan = VfsPlan::new(
            PathBuf::from("/var/games/skyrim_base"),
            PathBuf::from("/tmp/sandbox/mount"),
        )
        .with_lower_root(PathBuf::from("/opt/games/skyrim_dlc"))
        .with_layers(vec![PathBuf::from("/home/user/.mods/skse")]);

        let opts = plan.format_mount_options().unwrap();
        assert!(opts.starts_with("lowerdir="));
        let lowerdirs = opts.strip_prefix("lowerdir=").unwrap();
        let parts: Vec<&str> = lowerdirs.split(':').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts[0].ends_with("skse"));
        assert!(parts[1].ends_with("skyrim_base"));
        assert!(parts[2].ends_with("skyrim_dlc"));
    }

    #[test]
    fn test_vfs_plan_from_roots() {
        let plan = VfsPlan::from_roots(
            vec![
                PathBuf::from("/opt/storage/game_dlc"),
                PathBuf::from("/opt/storage/game_base"),
            ],
            PathBuf::from("/mnt/target"),
        )
        .unwrap()
        .with_layers(vec![PathBuf::from("/mods/mod_a")]);

        let opts = plan.format_mount_options().unwrap();
        let lowerdirs = opts.strip_prefix("lowerdir=").unwrap();
        let parts: Vec<&str> = lowerdirs.split(':').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts[0].ends_with("mod_a"));
        assert!(parts[1].ends_with("game_dlc"));
        assert!(parts[2].ends_with("game_base"));
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
