//-- ./src/lib.rs

#![doc = include_str!("../README.md")]

pub mod configuration;
pub mod database;
pub mod domain;
pub mod error;
pub mod middleware;
pub mod prelude;
pub mod reflections;
pub mod router;
pub mod rpc;
pub mod services;
pub mod startup;
pub mod telemetry;
pub mod utils;
