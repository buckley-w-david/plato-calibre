use anyhow::{Context, Error};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub base_url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub identifier: Option<String>,
    pub category: u64,
    pub item: u64,
    pub library: String,
    #[serde(default = "one")]
    pub log: u64,
}

// Is this really the correct way to put a default value?
fn one() -> u64 {
    1
}

pub fn load_toml<T, P: AsRef<Path>>(path: P) -> Result<T, Error>
where
    for<'a> T: Deserialize<'a>,
{
    let s = fs::read_to_string(path.as_ref())
        .with_context(|| format!("can't read file {}", path.as_ref().display()))?;
    toml::from_str(&s)
        .with_context(|| format!("can't parse TOML content from {}", path.as_ref().display()))
        .map_err(Into::into)
}
