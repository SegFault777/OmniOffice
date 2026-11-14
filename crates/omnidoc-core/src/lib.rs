// SPDX-License-Identifier: GPL-3.0-only
//! OmniDoc core: document model, editing engine, commands and format I/O.
//! No GUI code lives here; the UI crate only renders and forwards input.
pub mod cmd;
pub mod edit;
pub mod formats;
pub mod model;
pub use omni_common::{cfb, xml, zipio};
pub use omni_common::BUILD_ID as VERSION;
