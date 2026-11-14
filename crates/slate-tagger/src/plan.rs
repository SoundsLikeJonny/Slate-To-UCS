//! Deciding what would move, and refusing anything unsafe before a file is touched.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::MAX_FILENAME_BYTES;

/// One requested rename: the file as it is now, and the name it should get.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameRequest {
    /// The file as it is on disk.
    pub from: PathBuf,
    /// The desired file name, without a directory.
    pub to: String,
}

impl RenameRequest {
    /// A request from a path and a desired file name.
    #[must_use]
    pub fn new(from: impl Into<PathBuf>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

/// How strict the planner is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanOptions {
    /// Whether a desired name may contain a subdirectory.
    pub allow_subfolders: bool,
    /// Longest accepted file name in bytes.
    pub max_bytes: usize,
}

impl Default for PlanOptions {
    fn default() -> Self {
        Self {
            allow_subfolders: false,
            max_bytes: MAX_FILENAME_BYTES,
        }
    }
}

/// Why an entry will not be applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// The file already has the desired name.
    Unchanged,
    /// The desired name is empty, has a forbidden character, or is a reserved device name.
    InvalidName(String),
    /// The desired name is longer than the configured limit.
    TooLong(usize),
    /// The desired name contains a directory and subfolders are not allowed.
    Subfolder,
    /// Another file on disk already has the desired name.
    CollisionWithExisting(PathBuf),
    /// Another entry in the same plan wants the same name.
    CollisionWithinPlan(String),
    /// The same source file appears twice in the plan.
    DuplicateSource,
}

impl SkipReason {
    /// A stable, lower-snake-case identifier for the CLI and the UI.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::InvalidName(_) => "invalid_name",
            Self::TooLong(_) => "too_long",
            Self::Subfolder => "subfolder",
            Self::CollisionWithExisting(_) => "collision_with_existing",
            Self::CollisionWithinPlan(_) => "collision_within_plan",
            Self::DuplicateSource => "duplicate_source",
        }
    }

    /// A sentence a person can act on.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Unchanged => "already has this name".to_string(),
            Self::InvalidName(reason) => format!("invalid name: {reason}"),
            Self::TooLong(bytes) => format!("name is {bytes} bytes, over the limit"),
            Self::Subfolder => "name contains a directory".to_string(),
            Self::CollisionWithExisting(path) => {
                format!("{} already exists", path.display())
            }
            Self::CollisionWithinPlan(name) => {
                format!("another file in this plan also becomes {name}")
            }
            Self::DuplicateSource => "the same file appears twice".to_string(),
        }
    }
}

/// One line of a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanEntry {
    /// The file as it is on disk.
    pub from: PathBuf,
    /// The name it would get.
    pub to: PathBuf,
    /// Why it will not be applied, if it will not be.
    pub skip: Option<SkipReason>,
}

impl PlanEntry {
    /// Whether this entry would be applied.
    #[must_use]
    pub fn is_actionable(&self) -> bool {
        self.skip.is_none()
    }
}

/// The result of planning a directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The directory the plan applies to.
    pub directory: PathBuf,
    /// One entry per request, in request order.
    pub entries: Vec<PlanEntry>,
}

impl Plan {
    /// Whether every entry can be applied.
    ///
    /// An empty plan is actionable: applying it does nothing and succeeds.
    #[must_use]
    pub fn is_actionable(&self) -> bool {
        self.entries.iter().all(PlanEntry::is_actionable)
    }

    /// How many entries are blocked.
    #[must_use]
    pub fn blocked_count(&self) -> usize {
        self.entries.iter().filter(|e| !e.is_actionable()).count()
    }

    /// The entries that would be applied.
    pub fn actionable(&self) -> impl Iterator<Item = &PlanEntry> {
        self.entries.iter().filter(|e| e.is_actionable())
    }
}

/// Plan a set of renames inside one directory.
///
/// The plan is a pure function of the requests, the options and the directory listing,
/// so a dry run and the apply that follows it agree as long as nothing else changed.
#[must_use]
pub fn plan_renames(directory: &Path, requests: &[RenameRequest], options: &PlanOptions) -> Plan {
    let mut entries: Vec<PlanEntry> = Vec::with_capacity(requests.len());
    let mut seen_sources: HashSet<PathBuf> = HashSet::new();
    let mut claimed: HashSet<PathBuf> = HashSet::new();

    for request in requests {
        let from = if request.from.is_absolute() {
            request.from.clone()
        } else {
            directory.join(&request.from)
        };
        let target = directory.join(&request.to);
        let skip = if !seen_sources.insert(from.clone()) {
            Some(SkipReason::DuplicateSource)
        } else if let Some(reason) = validate_name(&request.to, options) {
            Some(reason)
        } else if target == from {
            Some(SkipReason::Unchanged)
        } else if !claimed.insert(target.clone()) {
            Some(SkipReason::CollisionWithinPlan(request.to.clone()))
        } else if target.exists() {
            Some(SkipReason::CollisionWithExisting(target.clone()))
        } else {
            None
        };
        entries.push(PlanEntry {
            from,
            to: target,
            skip,
        });
    }

    Plan {
        directory: directory.to_path_buf(),
        entries,
    }
}

/// Why a desired name is unacceptable, if it is.
fn validate_name(name: &str, options: &PlanOptions) -> Option<SkipReason> {
    if name.is_empty() {
        return Some(SkipReason::InvalidName("the name is empty".to_string()));
    }
    if name.len() > options.max_bytes {
        return Some(SkipReason::TooLong(name.len()));
    }
    let has_separator = name.contains('/') || name.contains('\\');
    if has_separator && !options.allow_subfolders {
        return Some(SkipReason::Subfolder);
    }
    if has_separator && name.split(['/', '\\']).any(|part| part == "..") {
        return Some(SkipReason::InvalidName(
            "the name escapes the directory".to_string(),
        ));
    }
    if let Some(bad) = name
        .chars()
        .find(|c| c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
    {
        return Some(SkipReason::InvalidName(format!(
            "the character {bad:?} is not allowed"
        )));
    }
    if name.ends_with(' ') || name.ends_with('.') {
        return Some(SkipReason::InvalidName(
            "the name ends with a space or a dot".to_string(),
        ));
    }
    let stem = name.split('.').next().unwrap_or(name);
    if is_reserved_device_name(stem) {
        return Some(SkipReason::InvalidName(format!(
            "{stem} is a reserved device name"
        )));
    }
    None
}

/// Whether a stem is one of the Windows device names that cannot be a file name.
fn is_reserved_device_name(stem: &str) -> bool {
    let upper = stem.to_ascii_uppercase();
    if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return true;
    }
    let bytes = upper.as_bytes();
    if bytes.len() == 4 && (upper.starts_with("COM") || upper.starts_with("LPT")) {
        return bytes[3].is_ascii_digit() && bytes[3] != b'0';
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir_with(names: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temp dir");
        for name in names {
            std::fs::write(dir.path().join(name), b"x").expect("write");
        }
        dir
    }

    #[test]
    fn a_clean_plan_is_actionable() {
        let dir = dir_with(&["a.wav"]);

        let plan = plan_renames(
            dir.path(),
            &[RenameRequest::new(dir.path().join("a.wav"), "b.wav")],
            &PlanOptions::default(),
        );

        assert!(plan.is_actionable());
        assert_eq!(plan.blocked_count(), 0);
        assert_eq!(plan.entries[0].to, dir.path().join("b.wav"));
    }

    #[test]
    fn an_empty_plan_is_actionable() {
        let dir = dir_with(&[]);

        let plan = plan_renames(dir.path(), &[], &PlanOptions::default());

        assert!(plan.is_actionable());
        assert_eq!(plan.entries.len(), 0);
    }

    #[test]
    fn an_unchanged_name_is_skipped() {
        let dir = dir_with(&["a.wav"]);

        let plan = plan_renames(
            dir.path(),
            &[RenameRequest::new(dir.path().join("a.wav"), "a.wav")],
            &PlanOptions::default(),
        );

        assert_eq!(plan.entries[0].skip, Some(SkipReason::Unchanged));
        assert!(!plan.is_actionable());
    }

    #[test]
    fn an_existing_target_is_a_collision() {
        let dir = dir_with(&["a.wav", "b.wav"]);

        let plan = plan_renames(
            dir.path(),
            &[RenameRequest::new(dir.path().join("a.wav"), "b.wav")],
            &PlanOptions::default(),
        );

        assert_eq!(
            plan.entries[0].skip,
            Some(SkipReason::CollisionWithExisting(dir.path().join("b.wav")))
        );
    }

    #[test]
    fn two_requests_for_one_name_collide_within_the_plan() {
        let dir = dir_with(&["a.wav", "b.wav"]);

        let plan = plan_renames(
            dir.path(),
            &[
                RenameRequest::new(dir.path().join("a.wav"), "c.wav"),
                RenameRequest::new(dir.path().join("b.wav"), "c.wav"),
            ],
            &PlanOptions::default(),
        );

        assert!(plan.entries[0].is_actionable());
        assert_eq!(
            plan.entries[1].skip,
            Some(SkipReason::CollisionWithinPlan("c.wav".to_string()))
        );
        assert_eq!(plan.blocked_count(), 1);
    }

    #[test]
    fn the_same_source_twice_is_a_duplicate() {
        let dir = dir_with(&["a.wav"]);

        let plan = plan_renames(
            dir.path(),
            &[
                RenameRequest::new(dir.path().join("a.wav"), "b.wav"),
                RenameRequest::new(dir.path().join("a.wav"), "c.wav"),
            ],
            &PlanOptions::default(),
        );

        assert_eq!(plan.entries[1].skip, Some(SkipReason::DuplicateSource));
    }

    #[test]
    fn an_empty_name_is_invalid() {
        let dir = dir_with(&["a.wav"]);

        let plan = plan_renames(
            dir.path(),
            &[RenameRequest::new(dir.path().join("a.wav"), "")],
            &PlanOptions::default(),
        );

        assert!(matches!(
            plan.entries[0].skip,
            Some(SkipReason::InvalidName(_))
        ));
    }

    #[test]
    fn a_forbidden_character_is_invalid() {
        let dir = dir_with(&["a.wav"]);

        for name in ["a:b.wav", "a|b.wav", "a?b.wav", "a*b.wav", "a\"b.wav"] {
            let plan = plan_renames(
                dir.path(),
                &[RenameRequest::new(dir.path().join("a.wav"), name)],
                &PlanOptions::default(),
            );
            assert!(
                matches!(plan.entries[0].skip, Some(SkipReason::InvalidName(_))),
                "{name} was accepted"
            );
        }
    }

    #[test]
    fn a_trailing_space_or_dot_is_invalid() {
        let dir = dir_with(&["a.wav"]);

        for name in ["a.wav ", "a."] {
            let plan = plan_renames(
                dir.path(),
                &[RenameRequest::new(dir.path().join("a.wav"), name)],
                &PlanOptions::default(),
            );
            assert!(
                matches!(plan.entries[0].skip, Some(SkipReason::InvalidName(_))),
                "{name} was accepted"
            );
        }
    }

    #[test]
    fn a_space_before_the_extension_is_allowed() {
        let dir = dir_with(&["a.wav"]);

        let plan = plan_renames(
            dir.path(),
            &[RenameRequest::new(dir.path().join("a.wav"), "a .wav")],
            &PlanOptions::default(),
        );

        assert!(plan.entries[0].is_actionable());
    }

    #[test]
    fn reserved_device_names_are_invalid() {
        let dir = dir_with(&["a.wav"]);

        for name in ["CON.wav", "nul.wav", "COM1.wav", "lpt9.wav"] {
            let plan = plan_renames(
                dir.path(),
                &[RenameRequest::new(dir.path().join("a.wav"), name)],
                &PlanOptions::default(),
            );
            assert!(
                matches!(plan.entries[0].skip, Some(SkipReason::InvalidName(_))),
                "{name} was accepted"
            );
        }
    }

    #[test]
    fn a_device_lookalike_is_allowed() {
        let dir = dir_with(&["a.wav"]);

        for name in ["CONSOLE.wav", "COM0.wav", "LPT.wav"] {
            let plan = plan_renames(
                dir.path(),
                &[RenameRequest::new(dir.path().join("a.wav"), name)],
                &PlanOptions::default(),
            );
            assert!(plan.entries[0].is_actionable(), "{name} was refused");
        }
    }

    #[test]
    fn a_subfolder_is_refused_by_default() {
        let dir = dir_with(&["a.wav"]);

        let plan = plan_renames(
            dir.path(),
            &[RenameRequest::new(dir.path().join("a.wav"), "sub/b.wav")],
            &PlanOptions::default(),
        );

        assert_eq!(plan.entries[0].skip, Some(SkipReason::Subfolder));
    }

    #[test]
    fn a_subfolder_is_allowed_when_opted_in() {
        let dir = dir_with(&["a.wav"]);
        let options = PlanOptions {
            allow_subfolders: true,
            ..PlanOptions::default()
        };

        let plan = plan_renames(
            dir.path(),
            &[RenameRequest::new(dir.path().join("a.wav"), "sub/b.wav")],
            &options,
        );

        assert!(plan.entries[0].is_actionable());
    }

    #[test]
    fn a_parent_escape_is_refused_even_when_subfolders_are_allowed() {
        let dir = dir_with(&["a.wav"]);
        let options = PlanOptions {
            allow_subfolders: true,
            ..PlanOptions::default()
        };

        let plan = plan_renames(
            dir.path(),
            &[RenameRequest::new(dir.path().join("a.wav"), "../b.wav")],
            &options,
        );

        assert!(matches!(
            plan.entries[0].skip,
            Some(SkipReason::InvalidName(_))
        ));
    }

    #[test]
    fn a_long_name_is_refused_with_its_length() {
        let dir = dir_with(&["a.wav"]);
        let long = "x".repeat(MAX_FILENAME_BYTES + 1);

        let plan = plan_renames(
            dir.path(),
            &[RenameRequest::new(dir.path().join("a.wav"), long.clone())],
            &PlanOptions::default(),
        );

        assert_eq!(plan.entries[0].skip, Some(SkipReason::TooLong(long.len())));
    }

    #[test]
    fn the_length_limit_is_configurable() {
        let dir = dir_with(&["a.wav"]);
        let options = PlanOptions {
            max_bytes: 4,
            ..PlanOptions::default()
        };

        let plan = plan_renames(
            dir.path(),
            &[RenameRequest::new(dir.path().join("a.wav"), "abcde")],
            &options,
        );

        assert_eq!(plan.entries[0].skip, Some(SkipReason::TooLong(5)));
    }

    #[test]
    fn every_skip_reason_has_a_distinct_kind_and_a_sentence() {
        let reasons = vec![
            SkipReason::Unchanged,
            SkipReason::InvalidName("x".to_string()),
            SkipReason::TooLong(1),
            SkipReason::Subfolder,
            SkipReason::CollisionWithExisting(PathBuf::from("/a")),
            SkipReason::CollisionWithinPlan("a".to_string()),
            SkipReason::DuplicateSource,
        ];
        let kinds: Vec<&str> = reasons.iter().map(SkipReason::kind).collect();
        let mut unique = kinds.clone();
        unique.sort_unstable();
        unique.dedup();

        assert_eq!(kinds.len(), unique.len(), "kinds: {kinds:?}");
        for reason in &reasons {
            assert!(!reason.describe().is_empty(), "{reason:?} has no sentence");
        }
    }

    #[test]
    fn a_relative_source_is_resolved_against_the_directory() {
        let dir = dir_with(&["a.wav"]);

        let plan = plan_renames(
            dir.path(),
            &[RenameRequest::new("a.wav", "b.wav")],
            &PlanOptions::default(),
        );

        assert_eq!(plan.entries[0].from, dir.path().join("a.wav"));
        assert!(plan.entries[0].is_actionable());
    }

    #[test]
    fn a_relative_source_that_matches_the_target_is_unchanged() {
        let dir = dir_with(&["a.wav"]);

        let plan = plan_renames(
            dir.path(),
            &[RenameRequest::new("a.wav", "a.wav")],
            &PlanOptions::default(),
        );

        assert_eq!(plan.entries[0].skip, Some(SkipReason::Unchanged));
    }

    #[test]
    fn actionable_filters_the_blocked_entries() {
        let dir = dir_with(&["a.wav", "b.wav"]);

        let plan = plan_renames(
            dir.path(),
            &[
                RenameRequest::new(dir.path().join("a.wav"), "c.wav"),
                RenameRequest::new(dir.path().join("b.wav"), "b.wav"),
            ],
            &PlanOptions::default(),
        );

        let names: Vec<&Path> = plan.actionable().map(|e| e.to.as_path()).collect();

        assert_eq!(names, vec![dir.path().join("c.wav").as_path()]);
    }
}
