// SPDX-License-Identifier: GPL-3.0-only
//! OmniDoc core: document model, editing engine, commands and format I/O.
//! No GUI code lives here; the UI crate only renders and forwards input.
pub mod cmd;
pub mod edit;
pub mod formats;
pub mod model;
pub mod xml;
pub mod zipio;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
