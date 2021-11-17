use serde::{Deserialize, Serialize};
use chrono::prelude::*;
use std::path::PathBuf;

use crate::utils::datetime_format;

#[derive(Debug, Deserialize, Serialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub kind: String,
    pub size: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Info {
    pub title: String,

    pub author: String,

    pub identifier: String,

    pub file: FileInfo,

    #[serde(with = "datetime_format")]
    pub added: DateTime<Utc>,
}
