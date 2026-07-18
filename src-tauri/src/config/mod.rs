//! Config-Persistenz (config.json).
//!
//! v1: `apiKey` wird im Klartext in `config.json` gespeichert.
//! v2-TODO: DPAPI-Verschlüsselung via `windows`-Crate oder
//! plattformübergreifend via `keyring`-Crate. Siehe DECISIONS.md
//! § Config-Storage-v1-plaintext.

pub mod io;
pub mod schema;

pub use io::{config_path, load_config, save_config};
pub use schema::{Config, ConfigError, EndpointConfig, ModelConfig};