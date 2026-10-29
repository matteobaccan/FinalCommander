//! The file operation engine: everything that touches the filesystem goes through here.
//!
//! Three phases, as in `docs/spec/03-file-operations.md`:
//!
//! 1. **plan** — expand the selection, count files and bytes, refuse the impossible up front;
//! 2. **execute** — run the units, asking the caller what to do about conflicts and errors;
//! 3. **settle** — apply deferred directory metadata and report what happened.
//!
//! The engine is synchronous and talks to the UI through the [`Reporter`] trait, so the runner
//! can drive it on a worker thread while the UI thread stays responsive, and tests can drive it
//! with a scripted reporter.

pub mod arcfs;
pub mod attrs;
pub mod fs;
pub mod link;
pub mod plan;
pub mod replace;
pub mod run;
pub mod spec;
pub mod sum;
pub mod sync;

pub use arcfs::ArcFs;
pub use attrs::{set_attributes, Change, Mode};
pub use fs::{free_name, part_path, CopyHow, Facts, JobFs, LocalFs, PART_SUFFIX};
pub use link::{create_link, LinkError, LinkKind};
pub use plan::{plan, Plan, PlanError, Unit, UnitKind};
pub use replace::{
    replace_bytes, Matcher, ReplaceError, ReplaceOptions, ReplacePass, Replaced, PATTERN_FILE_LIMIT,
};
pub use run::{execute, Outcome, Progress, Reporter};
pub use spec::{ConflictPolicy, Decision, JobKind, JobOptions, JobSpec, Phase, Preserve, Question};
pub use sum::Algorithm;
