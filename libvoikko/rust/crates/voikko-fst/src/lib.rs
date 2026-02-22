//! VFST (Voikko Finite State Transducer) engine.
//!
//! This crate provides loading and traversal of VFST binary transducer files,
//! supporting both unweighted and weighted variants. It is a Rust port of the
//! C++ FST engine in libvoikko (`libvoikko/src/fst/`).
//!
//! # Architecture
//!
//! - [`format`] -- Binary header parsing and validation
//! - [`transition`] -- Zero-copy transition struct layout
//! - [`symbols`] -- Symbol table (char-to-index and index-to-string mapping)
//! - [`flags`] -- Flag diacritic operations (P, C, U, R, D)
//! - [`config`] -- Traversal configuration (explicit DFS stack)
//! - [`unweighted`] -- Unweighted transducer loading and traversal
//! - [`weighted`] -- Weighted transducer loading and traversal

pub mod config;
pub mod flags;
pub mod format;
pub mod symbols;
pub mod transition;
pub mod unweighted;
pub mod weighted;

/// Error type for VFST parsing and loading.
#[derive(Debug, thiserror::Error)]
pub enum VfstError {
    #[error("invalid magic number in VFST header")]
    InvalidMagic,
    #[error("file too short: expected at least {expected} bytes, got {actual}")]
    TooShort { expected: usize, actual: usize },
    #[error("type mismatch: expected weighted={expected}, got weighted={actual}")]
    TypeMismatch { expected: bool, actual: bool },
    #[error("invalid symbol table: {0}")]
    InvalidSymbolTable(String),
    #[error("invalid flag diacritic: {0}")]
    InvalidFlagDiacritic(String),
    #[error("transition table alignment error")]
    AlignmentError,
}

/// Maximum number of outer-loop iterations in the traversal algorithm.
/// Acts as a safety limit to prevent infinite loops.
///
/// Origin: Transducer.hpp:57
pub const MAX_LOOP_COUNT: u32 = 100_000;

/// Trait for transducer traversal, abstracting over weighted/unweighted variants.
///
/// The `prepare` + `next` pattern is a coroutine-like interface: `prepare` sets up
/// the configuration for a new input, and each `next` call yields one output string.
pub trait Transducer {
    type Config;

    /// Prepare the configuration for traversing with the given input characters.
    ///
    /// Returns `true` if all input characters are known symbols.
    /// For unweighted transducers, unknown characters are mapped to a sentinel
    /// and traversal may still proceed (returns `false` but is usable).
    /// For weighted transducers, unknown characters cause an immediate `false` return
    /// and no traversal is possible.
    fn prepare(&self, config: &mut Self::Config, input: &[char]) -> bool;

    /// Yield the next output from the transducer.
    ///
    /// Returns `true` if an output was found, `false` if no more outputs exist
    /// (or if the loop limit was reached).
    fn next(&self, config: &mut Self::Config, output: &mut String) -> bool;
}
