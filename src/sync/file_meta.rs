use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMeta {
    pub path: PathBuf,
    pub downloaded_at: Option<chrono::DateTime<chrono::Utc>>,
    pub etag: Option<String>,
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
    pub imported_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl FileMeta {
    /// file_name is the name of the .tsv.gz file
    pub fn new(file_name: &PathBuf) -> Result<Self> {
        let meta_path = file_name.with_extension("json");
        let meta = match std::fs::read_to_string(&meta_path) {
            Ok(meta) => {
                let data: FileMeta = serde_json::from_str(&meta)?;
                assert_eq!(data.path, file_name.clone());
                data
            }
            Err(_) => FileMeta {
                path: file_name.clone(),
                downloaded_at: None,
                etag: None,
                last_modified: None,
                imported_at: None,
            },
        };

        Ok(meta)
    }

    pub fn save(&self) -> Result<()> {
        let meta_path = self.path.with_extension("json");
        let meta = serde_json::to_string_pretty(self)?;
        std::fs::write(&meta_path, meta)?;
        Ok(())
    }
}
