//! # Hypernovae
//! A feature-complete, extensible framework for creating custom,
//! high-performance Minecraft: Java Edition servers.
//!
//! This is ***HIGHLY EXPERIMENTAL*** and/or ***INCOMPLETE***!
//! Contributions are welcome on our [Github](https://github.com/noahyor/hypernovae).

/// Things related to the game
pub mod game;

/// Contains utilities for reading config files
pub mod config;

/// Handles the networking protocol
pub mod net;

/// Error utilities
pub mod error;

/// Tests
#[cfg(test)]
mod test;
