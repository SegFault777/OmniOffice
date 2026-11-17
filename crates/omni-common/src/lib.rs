// SPDX-License-Identifier: GPL-3.0-only
//! Code shared by OmniDoc, OmniCel, OmniShow and the OmniOffice launcher.
pub mod cfb;
pub mod config;
pub mod image;
pub mod xml;
pub mod zipio;
/// Build identifier: <year><ISO week><letter>, e.g. 202646b.
pub const BUILD_ID: &str = "202647a";
