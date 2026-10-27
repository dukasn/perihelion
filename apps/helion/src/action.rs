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

use peri_core::draft::SessionDraft;
use peri_core::manifest::Priority;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Mods,
    Conflicts,
    Downloads,
    Profiles,
}

impl Tab {
    pub fn index(&self) -> usize {
        match self {
            Tab::Overview => 0,
            Tab::Mods => 1,
            Tab::Conflicts => 2,
            Tab::Downloads => 3,
            Tab::Profiles => 4,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Tab::Overview,
            1 => Tab::Mods,
            2 => Tab::Conflicts,
            3 => Tab::Downloads,
            4 => Tab::Profiles,
            _ => Tab::Overview,
        }
    }

    pub fn next(&self) -> Self {
        Self::from_index((self.index() + 1) % 5)
    }

    pub fn prev(&self) -> Self {
        Self::from_index((self.index() + 4) % 5)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverviewFocus {
    CommandInput,
    RecentProfiles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModsFocus {
    Search,
    Categories,
    Table,
    Inspector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictsFocus {
    AutoResolve,
    OptionA,
    OptionB,
    ResolvedList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadsFocus {
    Input,
    Queue,
    Curated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfilesFocus {
    NewProfileInput,
    GameSelector,
    ProfileList,
    Inspector,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputValidation {
    Idle,
    Valid(String),
    Warning(String),
    Invalid(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Tick,
    Render,
    Quit,
    NextTab,
    PrevTab,
    SetTab(Tab),
    FocusNext,
    FocusPrev,
    SwitchPaneFocus,
    CursorUp,
    CursorDown,
    CursorPageUp,
    CursorPageDown,
    CursorHome,
    CursorEnd,
    ActivateFocused,
    ToggleMod,
    ToggleAllMods,
    CyclePriority,
    CyclePriorityReverse,
    MoveModUp,
    MoveModDown,
    ToggleInspector,
    StartSearch,
    EndSearch,
    SearchChar(char),
    SearchBackspace,
    ClearSearchAndFilters,
    FilterCategory(Option<String>),
    CycleCategory,
    CycleCategoryReverse,
    ResolveConflictChoice {
        index: usize,
        winner_id: String,
    },
    ToggleConflictWinner,
    AutoResolveConflicts,
    SelectProfile(usize),
    StartProfileInput,
    StartOverviewInput,
    CycleTargetGame,
    CycleTargetGameReverse,
    FixDiagnostic,
    StartDownload(String),
    StartDownloadInput,
    TogglePauseDownload,
    InstallCuratedPackage,
    InputChar(char),
    InputBackspace,
    InputDeleteWord,
    InputClear,
    SubmitInput,
    CancelInput,
    LaunchGame,
    ToggleHelp,
    CloseModal,

    // ----------------------------------------------------
    // Profile Management & Wizard Modals
    // ----------------------------------------------------
    OpenGameWizard,
    CloseGameWizard,
    OpenSteamImporter,
    CloseSteamImporter,
    SaveGameWizard,
    ImportSteamGame,
    ToggleWizardLocation,
    ToggleSteamImporterLocation,
    DeleteSelectedProfile,
    CycleWizardField,
    CycleWizardFieldReverse,

    // ----------------------------------------------------
    // Transactional Draft Lifecycle
    // ----------------------------------------------------
    StageToggle {
        mod_id: String,
        active: bool,
    },
    StagePriority {
        mod_id: String,
        priority: Priority,
    },
    StageConflictWinner {
        path: String,
        winning_mod: String,
    },
    CommitDraftToDisk,
    DiscardDraft,

    // ----------------------------------------------------
    // Asynchronous Engine Responses (From Tokio Workers)
    // ----------------------------------------------------
    DagSolved {
        ordered_mod_ids: Vec<String>,
        has_cycles: bool,
    },
    VfiScanCompleted {
        collision_count: usize,
    },

    // ----------------------------------------------------
    // Execution & Sandbox Lifecycle
    // ----------------------------------------------------
    LaunchGameTriggered,
    GameStarted {
        pid: u32,
        snapshot_id: String,
    },
    GameTerminated {
        exit_code: Option<i32>,
    },
    RestartGameRequested,

    // ----------------------------------------------------
    // Diagnostics, Error & Recovery
    // ----------------------------------------------------
    SessionRestored(Box<SessionDraft>),
    RecoveryConflictDetected {
        on_disk_hash: String,
        draft_hash: String,
    },
}
