use anyhow::Result;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::{io::Write, path::PathBuf};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
struct FileMeta {
    downloaded_at: chrono::DateTime<chrono::Utc>,
    etag: Option<String>,
    last_modified: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn ensure_file(file_path: &PathBuf, url: &str) -> Result<bool> {
    let meta_path = file_path.with_extension("json");
    let meta = match std::fs::read_to_string(&meta_path) {
        Ok(meta) => serde_json::from_str::<FileMeta>(&meta)?,
        Err(_) => FileMeta {
            downloaded_at: chrono::Utc::now(),
            etag: None,
            last_modified: None,
        },
    };

    let client = reqwest::Client::new();
    let mut request = client.get(url);
    if let Some(etag) = &meta.etag {
        request = request.header("If-None-Match", etag);
    }

    if let Some(last_modified) = &meta.last_modified {
        request = request.header("If-Modified-Since", last_modified.to_rfc2822());
    }

    let response = request.send().await?.error_for_status()?;
    match response.status() {
        reqwest::StatusCode::OK => {
            info!("Downloading {:?}", file_path);
            let etag = response
                .headers()
                .get("ETag")
                .map(|v| v.to_str().unwrap().to_string());
            let last_modified = response.headers().get("Last-Modified").map(|v| {
                chrono::DateTime::parse_from_rfc2822(v.to_str().unwrap())
                    .unwrap()
                    .with_timezone(&chrono::Utc)
            });

            let mut file = std::fs::File::create(&file_path)?;
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                file.write_all(&chunk?)?;
            }

            let meta = FileMeta {
                downloaded_at: chrono::Utc::now(),
                etag,
                last_modified,
            };

            let meta = serde_json::to_string(&meta)?;
            std::fs::write(&meta_path, meta)?;
            info!("Downloaded {:?}", file_path);
            Ok(true)
        }
        reqwest::StatusCode::NOT_MODIFIED => {
            info!("{:?} is up to date", file_path);
            Ok(false)
        }
        code => Err(anyhow::anyhow!(
            "Unexpected status code for {:?}: {}",
            file_path,
            code
        )),
    }
}
