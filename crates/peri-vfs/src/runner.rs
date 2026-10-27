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

//! Pluggable Runner Pipeline
//!
//! Provides an extensible, composable execution pipeline supporting:
//! - Native Linux binaries
//! - System and custom Wine environments
//! - Valve Proton compatibility runners
//! - Wrapper layers (gamemoderun, mangohud, gamescope, debugger/profiler chains)
//! - Automated sandbox boundary generation (whitelisting prefixes, runtime sockets, and temp paths)

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::proton::ProtonLaunch;
use crate::{SandboxPolicy, VfsError, VfsPlan};

/// Splits a command line string into tokens taking quotes and escapes into account.
pub fn parse_command_line(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape = false;

    for ch in input.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }

        match ch {
            '\\' if !in_single_quote => {
                escape = true;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            c if c.is_whitespace() && !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    tokens.push(current);
                    current = String::new();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

/// Standalone wrapper command prepended ahead of runner and target executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandWrapper {
    pub binary: String,
    pub args: Vec<String>,
}

impl CommandWrapper {
    pub fn new(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
            args: Vec::new(),
        }
    }

    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn with_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Feral GameMode optimization wrapper (`gamemoderun`).
    pub fn gamemode() -> Self {
        Self::new("gamemoderun")
    }

    /// MangoHud performance monitor wrapper (`mangohud`).
    pub fn mangohud() -> Self {
        Self::new("mangohud")
    }

    /// Gamescope micro-compositor wrapper with automatic `--` delimiter insertion.
    pub fn gamescope(custom_opts: Option<&str>) -> Self {
        let mut wrapper = Self::new("gamescope");
        if let Some(opts) = custom_opts {
            let tokens = parse_command_line(opts);
            wrapper = wrapper.with_args(tokens);
        }
        if wrapper.args.last().map(|s| s.as_str()) != Some("--") {
            wrapper = wrapper.with_arg("--");
        }
        wrapper
    }

    /// Parses a wrapper command line string into a `CommandWrapper`.
    pub fn parse(s: &str) -> Option<Self> {
        let tokens = parse_command_line(s);
        if tokens.is_empty() {
            return None;
        }
        let mut iter = tokens.into_iter();
        let binary = iter.next()?;
        Some(Self {
            binary,
            args: iter.collect(),
        })
    }

    /// Converts this wrapper into its individual command-line tokens.
    pub fn to_tokens(&self) -> Vec<String> {
        let mut tokens = Vec::with_capacity(1 + self.args.len());
        tokens.push(self.binary.clone());
        tokens.extend(self.args.iter().cloned());
        tokens
    }
}

/// Underlying compatibility runner translation backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerKind {
    /// Native Linux execution without translation.
    Native,
    /// System or custom Wine runner.
    Wine { runner: PathBuf, prefix: PathBuf },
    /// Valve Proton compatibility tool.
    Proton(ProtonLaunch),
    /// Custom runner executable with prefix arguments.
    Custom { runner: PathBuf, args: Vec<String> },
}

/// Fully configured execution pipeline ready for isolated namespace execution or dry-run inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerPipeline {
    pub target_executable: PathBuf,
    pub target_args: Vec<String>,
    pub runner: RunnerKind,
    pub wrappers: Vec<CommandWrapper>,
    pub env_vars: Vec<(String, String)>,
    pub working_dir: Option<PathBuf>,
}

impl RunnerPipeline {
    /// Creates a native Linux runner pipeline for the given target executable.
    pub fn new(target_executable: impl Into<PathBuf>) -> Self {
        Self {
            target_executable: target_executable.into(),
            target_args: Vec::new(),
            runner: RunnerKind::Native,
            wrappers: Vec::new(),
            env_vars: Vec::new(),
            working_dir: None,
        }
    }

    /// Convenience constructor for native execution.
    pub fn native(target_executable: impl Into<PathBuf>) -> Self {
        Self::new(target_executable)
    }

    /// Constructs a Wine runner pipeline using default `wine` binary.
    pub fn wine(target_executable: impl Into<PathBuf>, prefix: impl Into<PathBuf>) -> Self {
        Self::wine_with_runner(target_executable, PathBuf::from("wine"), prefix)
    }

    /// Constructs a Wine runner pipeline with an explicit wine runner binary.
    pub fn wine_with_runner(
        target_executable: impl Into<PathBuf>,
        runner: impl Into<PathBuf>,
        prefix: impl Into<PathBuf>,
    ) -> Self {
        let mut pipeline = Self::new(target_executable);
        pipeline.runner = RunnerKind::Wine {
            runner: runner.into(),
            prefix: prefix.into(),
        };
        pipeline
    }

    /// Constructs a Proton runner pipeline from an active `ProtonLaunch` configuration.
    pub fn proton(proton: ProtonLaunch, target_executable: impl Into<PathBuf>) -> Self {
        let mut pipeline = Self::new(target_executable);
        pipeline.runner = RunnerKind::Proton(proton);
        pipeline
    }

    /// Constructs a custom runner pipeline.
    pub fn custom(
        runner: impl Into<PathBuf>,
        runner_args: Vec<String>,
        target_executable: impl Into<PathBuf>,
    ) -> Self {
        let mut pipeline = Self::new(target_executable);
        pipeline.runner = RunnerKind::Custom {
            runner: runner.into(),
            args: runner_args,
        };
        pipeline
    }

    pub fn with_runner(mut self, runner: RunnerKind) -> Self {
        self.runner = runner;
        self
    }

    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.target_args.push(arg.into());
        self
    }

    pub fn with_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.target_args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn with_wrapper(mut self, wrapper: CommandWrapper) -> Self {
        self.wrappers.push(wrapper);
        self
    }

    pub fn with_wrappers(mut self, wrappers: impl IntoIterator<Item = CommandWrapper>) -> Self {
        self.wrappers.extend(wrappers);
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.env_vars.push((key.into(), val.into()));
        self
    }

    pub fn with_envs(
        mut self,
        envs: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        for (k, v) in envs {
            self.env_vars.push((k.into(), v.into()));
        }
        self
    }

    pub fn with_working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Returns a short machine-readable identifier for the active runner kind.
    pub fn runner_name(&self) -> &'static str {
        match &self.runner {
            RunnerKind::Native => "native",
            RunnerKind::Wine { .. } => "wine",
            RunnerKind::Proton(_) => "proton",
            RunnerKind::Custom { .. } => "custom",
        }
    }

    /// Returns the binary names of all configured wrappers in order.
    pub fn wrapper_names(&self) -> Vec<String> {
        self.wrappers.iter().map(|w| w.binary.clone()).collect()
    }

    /// Assembles all injected environment variables across runner and custom entries.
    pub fn collected_env_vars(&self) -> Vec<(String, String)> {
        let mut vars = Vec::new();

        match &self.runner {
            RunnerKind::Native => {}
            RunnerKind::Wine { prefix, .. } => {
                vars.push((
                    "WINEPREFIX".to_string(),
                    prefix.to_string_lossy().into_owned(),
                ));
            }
            RunnerKind::Proton(proton) => {
                for (k, v) in &proton.env_vars {
                    vars.push((k.clone(), v.clone()));
                }
            }
            RunnerKind::Custom { .. } => {}
        }

        for (k, v) in &self.env_vars {
            if let Some(pos) = vars.iter().position(|(existing_k, _)| existing_k == k) {
                vars[pos] = (k.clone(), v.clone());
            } else {
                vars.push((k.clone(), v.clone()));
            }
        }

        vars
    }

    /// Assembles the complete command-line argv tokens: wrappers -> runner -> executable -> args.
    pub fn build_argv(&self) -> Vec<String> {
        let mut argv = Vec::new();

        // 1. Prepend wrapper command chains
        for wrapper in &self.wrappers {
            argv.extend(wrapper.to_tokens());
        }

        // 2. Compatibility runner binary and runner flags
        match &self.runner {
            RunnerKind::Native => {
                argv.push(self.target_executable.to_string_lossy().into_owned());
            }
            RunnerKind::Wine { runner, .. } => {
                argv.push(runner.to_string_lossy().into_owned());
                argv.push(self.target_executable.to_string_lossy().into_owned());
            }
            RunnerKind::Proton(proton) => {
                argv.push(proton.runner.to_string_lossy().into_owned());
                argv.push("run".to_string());
                argv.push(self.target_executable.to_string_lossy().into_owned());
            }
            RunnerKind::Custom { runner, args } => {
                argv.push(runner.to_string_lossy().into_owned());
                argv.extend(args.iter().cloned());
                argv.push(self.target_executable.to_string_lossy().into_owned());
            }
        }

        // 3. Target arguments
        argv.extend(self.target_args.iter().cloned());

        argv
    }

    /// Constructs a runnable `std::process::Command` configured with the pipeline's argv and environment.
    pub fn build_command(&self, default_working_dir: &Path) -> Result<Command, VfsError> {
        let argv = self.build_argv();
        if argv.is_empty() {
            return Err(VfsError::ExecutionFailed(
                "RunnerPipeline is empty; no command specified".to_string(),
            ));
        }

        let mut cmd = Command::new(&argv[0]);
        if argv.len() > 1 {
            cmd.args(&argv[1..]);
        }

        let work_dir = self.working_dir.as_deref().unwrap_or(default_working_dir);
        cmd.current_dir(work_dir);

        for (k, v) in self.collected_env_vars() {
            cmd.env(k, v);
        }

        Ok(cmd)
    }

    /// Generates a tailored `SandboxPolicy` whitelisting runner prefixes and user runtime sockets.
    pub fn build_sandbox_policy(&self, _plan: &VfsPlan) -> SandboxPolicy {
        let mut policy = SandboxPolicy::default();

        match &self.runner {
            RunnerKind::Native => {}
            RunnerKind::Wine { prefix, .. } => {
                policy = policy.with_writable_path(prefix.clone());
            }
            RunnerKind::Proton(proton) => {
                policy = policy
                    .with_writable_path(proton.compat_data_path.clone())
                    .with_writable_path(proton.wine_prefix.clone());
            }
            RunnerKind::Custom { .. } => {}
        }

        // Automatically whitelist XDG_RUNTIME_DIR if set and exists for graphics/audio sockets
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let path = PathBuf::from(runtime_dir);
            if path.exists() {
                policy = policy.with_writable_path(path);
            }
        }

        policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_line_tokens() {
        let tokens = parse_command_line("gamescope -w 1920 -h 1080 --");
        assert_eq!(tokens, vec!["gamescope", "-w", "1920", "-h", "1080", "--"]);

        let quoted = parse_command_line("tool --name \"Cyberpunk 2077\" --flag 'val with space'");
        assert_eq!(
            quoted,
            vec![
                "tool",
                "--name",
                "Cyberpunk 2077",
                "--flag",
                "val with space"
            ]
        );

        let empty = parse_command_line("    ");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_command_wrapper_constructors() {
        let gm = CommandWrapper::gamemode();
        assert_eq!(gm.binary, "gamemoderun");
        assert!(gm.args.is_empty());

        let gs = CommandWrapper::gamescope(Some("-w 1920 -h 1080"));
        assert_eq!(gs.binary, "gamescope");
        assert_eq!(gs.args, vec!["-w", "1920", "-h", "1080", "--"]);

        let gs_with_delimiter = CommandWrapper::gamescope(Some("-w 1920 --"));
        assert_eq!(gs_with_delimiter.args, vec!["-w", "1920", "--"]);

        let parsed = CommandWrapper::parse("mangohud --dlsym").unwrap();
        assert_eq!(parsed.binary, "mangohud");
        assert_eq!(parsed.args, vec!["--dlsym"]);
    }

    #[test]
    fn test_native_pipeline_argv() {
        let pipeline = RunnerPipeline::native("./bin/game")
            .with_args(["--flag", "value"])
            .with_env("CUSTOM", "1");

        let argv = pipeline.build_argv();
        assert_eq!(argv, vec!["./bin/game", "--flag", "value"]);
        assert_eq!(pipeline.runner_name(), "native");

        let envs = pipeline.collected_env_vars();
        assert!(envs.contains(&("CUSTOM".to_string(), "1".to_string())));
    }

    #[test]
    fn test_wine_pipeline_with_wrappers() {
        let pipeline = RunnerPipeline::wine("game.exe", "/home/user/.wine")
            .with_arg("-novid")
            .with_wrapper(CommandWrapper::gamemode())
            .with_wrapper(CommandWrapper::gamescope(Some("-w 2560 -h 1440")));

        let argv = pipeline.build_argv();
        assert_eq!(
            argv,
            vec![
                "gamemoderun",
                "gamescope",
                "-w",
                "2560",
                "-h",
                "1440",
                "--",
                "wine",
                "game.exe",
                "-novid"
            ]
        );
        assert_eq!(pipeline.runner_name(), "wine");
        assert_eq!(
            pipeline.wrapper_names(),
            vec!["gamemoderun".to_string(), "gamescope".to_string()]
        );

        let envs = pipeline.collected_env_vars();
        assert!(envs.contains(&("WINEPREFIX".to_string(), "/home/user/.wine".to_string())));

        let plan = VfsPlan::new(PathBuf::from("/games/base"), PathBuf::from("/mnt/target"));
        let policy = pipeline.build_sandbox_policy(&plan);
        assert!(policy
            .writable_paths
            .contains(&PathBuf::from("/home/user/.wine")));
    }

    #[test]
    fn test_proton_pipeline_argv_and_policy() {
        let proton = ProtonLaunch {
            runner: PathBuf::from("/steam/proton"),
            compat_data_path: PathBuf::from("/steam/compatdata/1086940"),
            wine_prefix: PathBuf::from("/steam/compatdata/1086940/pfx"),
            steam_client_path: PathBuf::from("/steam"),
            env_vars: vec![
                (
                    "STEAM_COMPAT_DATA_PATH".to_string(),
                    "/steam/compatdata/1086940".to_string(),
                ),
                (
                    "WINEPREFIX".to_string(),
                    "/steam/compatdata/1086940/pfx".to_string(),
                ),
            ],
        };

        let pipeline = RunnerPipeline::proton(proton, "bg3.exe")
            .with_arg("-dx11")
            .with_wrapper(CommandWrapper::mangohud());

        let argv = pipeline.build_argv();
        assert_eq!(
            argv,
            vec!["mangohud", "/steam/proton", "run", "bg3.exe", "-dx11"]
        );
        assert_eq!(pipeline.runner_name(), "proton");

        let plan = VfsPlan::new(PathBuf::from("/games/base"), PathBuf::from("/mnt/target"));
        let policy = pipeline.build_sandbox_policy(&plan);
        assert!(policy
            .writable_paths
            .contains(&PathBuf::from("/steam/compatdata/1086940")));
        assert!(policy
            .writable_paths
            .contains(&PathBuf::from("/steam/compatdata/1086940/pfx")));
    }
}
