use crate::sync::file_meta::FileMeta;
use anyhow::Result;
use futures::StreamExt;
use std::io::Write;

pub async fn ensure_file(meta: &mut FileMeta, url: &str) -> Result<bool> {
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
            meta.etag = response
                .headers()
                .get("ETag")
                .map(|v| v.to_str().unwrap().to_string());

            meta.last_modified = response.headers().get("Last-Modified").map(|v| {
                chrono::DateTime::parse_from_rfc2822(v.to_str().unwrap())
                    .unwrap()
                    .with_timezone(&chrono::Utc)
            });

            if meta.path.exists() {
                let content_length = response
                    .headers()
                    .get("Content-Length")
                    .map(|v| v.to_str().unwrap().parse::<u64>().unwrap());

                if let Some(content_length) = content_length {
                    let disk_size = meta.path.metadata().unwrap().len();
                    if disk_size == content_length {
                        tracing::debug!(
                            file = ?meta.path,
                            "skipping download, file on disk matches remote"
                        );
                        meta.save()?;
                        return Ok(false);
                    }
                }
            }

            let mut file = std::fs::File::create(&meta.path)?;
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                file.write_all(&chunk?)?;
            }

            meta.imported_at = None;
            meta.save()?;
            tracing::info!(file = ?meta.path, "downloaded file");
            Ok(true)
        }
        reqwest::StatusCode::NOT_MODIFIED => {
            tracing::info!(file = ?meta.path, "skipping download, remote confirms our version is up to date");
            Ok(false)
        }
        code => Err(anyhow::anyhow!(
            "Unexpected status code for {:?}: {}",
            meta.path,
            code
        )),
    }
}
