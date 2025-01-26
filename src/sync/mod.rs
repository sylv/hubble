use crate::sync::{ensure_file::ensure_file, import_file::import_file, importers::get_importers};
use anyhow::Result;
use importer::ImporterScheduling;
use std::{collections::HashMap, path::PathBuf};
use tantivy::Index;
use tracing::info;
use update_index::update_index;

mod ensure_file;
mod import_file;
mod importer;
mod importers;
mod nullable;
mod update_index;

pub async fn sync_data(data_dir: &PathBuf, index: &Index, pool: &sqlx::SqlitePool) -> Result<()> {
    let start = std::time::Instant::now();
    let importers = get_importers();
    let cache_dir = data_dir.join("cache");

    // ensure data dir exits
    std::fs::create_dir_all(&cache_dir).unwrap();

    // ensure all files are downloaded and up to date in parallel
    let mut handles: HashMap<&str, tokio::task::JoinHandle<bool>> = HashMap::new();
    for importer in &importers {
        let importer_name = importer.get_name();
        let file_path = cache_dir.join(importer_name);
        let file_url = importer.get_url().to_string();

        let handle = tokio::spawn(async move { ensure_file(&file_path, &file_url).await.unwrap() });

        handles.insert(importer_name, handle);
    }

    let mut was_changed: HashMap<String, bool> = HashMap::new();
    for (name, handle) in handles.iter_mut() {
        was_changed.insert(name.to_string(), handle.await.unwrap());
    }

    // if any file changes, we have to re-import basics to make sure
    // known_ids contains all the ids in the csv.
    // todo: if basics did not change, scan through without inserting
    let was_any_changed = was_changed.values().any(|v| *v);
    if was_any_changed {
        let basics_importer = importers
            .iter()
            .find(|i| i.get_scheduling() == ImporterScheduling::IsBasics)
            .unwrap();

        let name = basics_importer.get_name();
        was_changed.insert(name.to_string(), true);
    }

    // known_ids avoids foreign key constraint issues
    // sometimes rows show up with references that don't exist
    // (either imdb includes them in the data, or its due to us discarding some rows)
    // so we just store all the IDs we've seen during importing.
    let mut known_ids = roaring::RoaringBitmap::new();
    for importer in &importers {
        let was_changed = was_changed.get(importer.get_name()).unwrap();
        if !was_changed {
            continue;
        }

        import_file(
            importer,
            &mut known_ids,
            &cache_dir.join(importer.get_name()),
            &pool,
        )
        .await
        .unwrap();
    }

    if was_any_changed {
        info!("Updating index");
        update_index(index, pool).await?;
    }

    let elapsed = start.elapsed();
    info!("Synced all data in {:?}", elapsed);
    Ok(())
}
