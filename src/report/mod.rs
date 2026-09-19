pub mod csv;
pub mod json;
pub mod text;

use serde::Serialize;

use anyhow::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
    Csv,
}

pub fn json<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string_pretty(value)?)
}
