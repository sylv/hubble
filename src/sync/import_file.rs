use anyhow::Result;
use async_compression::tokio::bufread::GzipDecoder;
use csv_async::AsyncReaderBuilder;
use futures::StreamExt;
use itertools::Itertools;
use roaring::RoaringBitmap;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tracing::{debug, info, warn};

use super::importer::Importer;

static MAX_BIND_VALUES: usize = 32766;
static MAX_BATCH_SIZE: usize = 10000;

pub async fn import_file(
    importer: &Box<dyn Importer>,
    known_ids: &mut RoaringBitmap,
    file_path: &PathBuf,
    pool: &SqlitePool,
) -> Result<()> {
    let start = std::time::Instant::now();
    let file = tokio::fs::File::open(&file_path).await?;
    let decompressor = GzipDecoder::new(tokio::io::BufReader::new(file));
    let mut reader = AsyncReaderBuilder::new()
        .delimiter(b'\t')
        .create_reader(decompressor);

    let headers = reader.headers().await?.clone();
    let batch_size = {
        let bind_count = importer.get_bind_count();
        (MAX_BIND_VALUES / bind_count).min(MAX_BATCH_SIZE)
    };

    info!(
        "Importing {} using batch size of {}",
        importer.get_name(),
        batch_size
    );

    let mut stream = reader.records().chunks(batch_size);
    let mut tx = pool.begin().await?;
    while let Some(batch) = stream.next().await {
        let rows = batch
            .into_iter()
            .filter_map(|row| {
                let row = row.ok();
                if row.is_none() {
                    warn!("Ignoring row that failed to parse");
                }
                row
            })
            .collect_vec();
        importer
            .write_batch(known_ids, &headers, rows, &mut tx)
            .await?;
    }

    debug!("Committing batch for {}", importer.get_name());
    tx.commit().await?;
    info!("Imported {} in {:?}", importer.get_name(), start.elapsed());
    Ok(())
}
