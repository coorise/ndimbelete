//! Domain models — plain data structs shared by commands and services.
//!
//! Think of these like TypeScript interfaces: they describe the shape of
//! data going to/from the frontend via `serde`.

#![allow(dead_code)]

mod cotisation;
mod member;
mod period;
mod role;
mod settings;
mod staff;

pub use cotisation::*;
pub use member::*;
pub use period::*;
pub use role::*;
pub use settings::*;
pub use staff::*;
