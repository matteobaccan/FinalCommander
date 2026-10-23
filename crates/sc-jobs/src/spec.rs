use sc_core::VPath;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobKind {
    Copy,
    Move,
    Delete,
    Mkdir,
}

impl JobKind {
    pub fn as_str(self) -> &'static str {
        match self {
            JobKind::Copy => "copy",
            JobKind::Move => "move",
            JobKind::Delete => "delete",
            JobKind::Mkdir => "mkdir",
        }
    }

    /// Does this job write into a target the user has to name?
    pub fn needs_target(self) -> bool {
        matches!(self, JobKind::Copy | JobKind::Move | JobKind::Mkdir)
    }
}

/// What to do when the destination already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConflictPolicy {
    /// Ask the user, once per conflict (the default).
    #[default]
    Ask,
    Overwrite,
    /// Overwrite only when the source is newer than the destination.
    OverwriteIfNewer,
    Skip,
    /// Give the copy a free name (`name (2).txt`).
    Rename,
}

/// Which metadata to carry over. Mode is Unix-only and ignored elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preserve {
    pub mtime: bool,
    pub mode: bool,
}

impl Default for Preserve {
    fn default() -> Self {
        Preserve {
            mtime: true,
            mode: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobOptions {
    pub on_conflict: ConflictPolicy,
    pub preserve: Preserve,
    /// Copy what a symlink points at instead of the link itself.
    pub follow_symlinks: bool,
    pub buffer: usize,
    /// Plan and report, change nothing.
    pub dry_run: bool,
}

impl Default for JobOptions {
    fn default() -> Self {
        JobOptions {
            on_conflict: ConflictPolicy::Ask,
            preserve: Preserve::default(),
            follow_symlinks: false,
            buffer: 256 * 1024,
            dry_run: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSpec {
    pub kind: JobKind,
    /// What the user had selected, before expansion.
    pub sources: Vec<VPath>,
    pub target: Option<VPath>,
    pub options: JobOptions,
}

impl JobSpec {
    pub fn new(kind: JobKind, sources: Vec<VPath>, target: Option<VPath>) -> Self {
        JobSpec {
            kind,
            sources,
            target,
            options: JobOptions::default(),
        }
    }

    pub fn with_options(mut self, options: JobOptions) -> Self {
        self.options = options;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Planning,
    Running,
    Settling,
    Done,
}

/// A decision point: the engine stops and asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Question {
    Conflict {
        src: VPath,
        dst: VPath,
        src_size: u64,
        dst_size: u64,
        src_newer: bool,
    },
    Error {
        path: VPath,
        error: String,
    },
}

/// The answer. `*All` answers are remembered for the rest of the job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Overwrite,
    OverwriteAll,
    Skip,
    SkipAll,
    /// Only valid for a conflict: write under this name in the same directory.
    Rename(String),
    /// Only valid for an error: try the same unit again.
    Retry,
    Cancel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_know_whether_they_need_a_target() {
        assert!(JobKind::Copy.needs_target());
        assert!(JobKind::Move.needs_target());
        assert!(JobKind::Mkdir.needs_target());
        assert!(!JobKind::Delete.needs_target());
    }

    #[test]
    fn defaults_are_the_cautious_ones() {
        let o = JobOptions::default();
        assert_eq!(o.on_conflict, ConflictPolicy::Ask);
        assert!(o.preserve.mtime && o.preserve.mode);
        assert!(
            !o.follow_symlinks,
            "a symlink is copied as a link by default"
        );
        assert!(!o.dry_run);
    }
}
