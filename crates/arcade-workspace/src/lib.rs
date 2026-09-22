//! Workspace services shared by desktop and headless applications.

/// A conservative default used until configurable huge-file policies land.
pub const DEFAULT_HUGE_FILE_THRESHOLD_BYTES: usize = 50 * 1024 * 1024;
