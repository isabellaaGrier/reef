//! Reef — bash compatibility layer for fish shell.
//!
//! Provides bash detection, translation, passthrough execution, and a
//! persistent bash coprocess daemon for seamless bash usage from fish.
//!
//! # Modules
//!
//! - [`detect`] — fast heuristic for identifying bash-specific syntax
//! - [`translate`] — bash-to-fish translation via AST
//! - [`parser`] — recursive-descent bash parser (produces [`ast`] nodes)
//! - [`ast`] — zero-copy abstract syntax tree types
//! - [`passthrough`] — bash subprocess execution with environment diffing
//! - [`daemon`] — persistent bash coprocess over a Unix domain socket
//! - [`env_diff`] — environment snapshot capture and diffing
//! - [`state`] — state file persistence for exported variables

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]
// Targeted pedantic allows — each justified:
#![allow(clippy::wildcard_imports)] // `use crate::ast::*` is intentional for AST types
#![allow(clippy::too_many_lines)] // parser/translator functions are inherently long
#![allow(clippy::items_after_statements)] // local structs near their usage is clearer

pub mod ast;
pub mod daemon;
pub mod detect;
pub mod env_diff;
pub mod lexer;
pub mod parser;
pub mod passthrough;
pub mod state;
pub mod translate;
