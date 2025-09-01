use super::importers::Importer;
use crate::sync::file_meta::FileMeta;
use anyhow::Result;
use async_compression::tokio::bufread::GzipDecoder;
use csv_async::AsyncReaderBuilder;
use futures::StreamExt;
use itertools::Itertools;
use sqlx::SqlitePool;
use std::time::Instant;

static MAX_BIND_VALUES: usize = 32766;
static MAX_BATCH_SIZE: usize = 10000;

pub async fn import_file(
    pool: &SqlitePool,
    importer: &Box<dyn Importer>,
    meta: &mut FileMeta,
) -> Result<()> {
    let file = tokio::fs::File::open(&meta.path).await?;
    let decompressor = GzipDecoder::new(tokio::io::BufReader::new(file));
    let mut reader = AsyncReaderBuilder::new()
        .has_headers(true)
        .delimiter(b'\t')
        .quoting(false)
        .create_reader(decompressor);

    let headers = reader.headers().await?.clone();
    let batch_size = {
        let bind_count = importer.get_bind_count();
        (MAX_BIND_VALUES / bind_count).min(MAX_BATCH_SIZE)
    };

    let name = importer.get_name();
    tracing::info!(file = name, "importing using batch size of {batch_size}");

    let mut stream = reader.records().chunks(batch_size);
    let mut done = 0;
    let mut last_log = Instant::now();
    let start = Instant::now();
    while let Some(batch) = stream.next().await {
        done += batch.len();
        let rows = batch.into_iter().map(|row| row.unwrap()).collect_vec();
        importer.write_batch(pool, &headers, rows).await?;
        if last_log.elapsed().as_secs() > 5 {
            let per_sec = done as f64 / start.elapsed().as_secs_f64();
            tracing::info!(
                file = name,
                "importing at {:.2} rows/sec, total: {done}",
                per_sec,
            );
            last_log = Instant::now();
        }
    }

    meta.imported_at = Some(chrono::Utc::now());
    meta.save()?;

    tracing::info!(file = name, "imported in {:?}", start.elapsed());
    Ok(())
}
