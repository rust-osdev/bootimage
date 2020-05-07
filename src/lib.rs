//! Provides functions to create a bootable OS image from a kernel binary.
//!
//! This crate is mainly built as a binary tool. Run `cargo install bootimage` to install it.

#![warn(missing_docs)]

pub mod args;
pub mod builder;
pub mod config;
pub mod run;

/// Contains help messages for the command line application.
pub mod help;
