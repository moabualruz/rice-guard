//! File I/O helpers for writing output files.
//!
//! Implements the individual write functions for each output file:
//! - `issues.json` — all issues, WSJF-sorted
//! - `issues-fixable.json` — auto_fixable = true subset
//! - `issues-remaining.json` — auto_fixable = false subset
//! - `summary.json` — aggregated scan statistics
//! - `summary.txt` — human-readable ASCII table
//!
//! Full implementation in Plan 03-04.
