use crate::sync::{
    ensure_file::ensure_file, file_meta::FileMeta, import_file::import_file,
    importers::get_importers,
};
use anyhow::Result;
use futures::future::try_join_all;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};
use tokio::{
    sync::{Mutex, Notify},
    task::JoinHandle,
};

mod ensure_file;
mod file_meta;
mod import_file;
mod importers;
mod nullable;

pub async fn sync_data(data_dir: &PathBuf, pool: &sqlx::SqlitePool) -> Result<()> {
    let importers = get_importers();
    let cache_dir = data_dir.join("cache");

    // ensure data dir exits
    std::fs::create_dir_all(&cache_dir).unwrap();

    let mut tasks: Vec<JoinHandle<Result<()>>> = Vec::new();
    let basics_done = Arc::new(Mutex::new((false, Arc::new(Notify::new()))));
    let needs_search_update = Arc::new(AtomicBool::new(false));
    for importer in importers {
        let pool = pool.clone();
        let basics_done = basics_done.clone();
        let cache_dir = cache_dir.clone();
        let needs_search_update = needs_search_update.clone();
        tasks.push(tokio::spawn(async move {
            let file_name = importer.get_name();
            let file_path = cache_dir.join(file_name);
            let mut meta = FileMeta::new(&file_path)?;

            ensure_file(&mut meta, importer.get_url()).await?;

            let is_basics = file_name == "title.basics.tsv.gz";
            let is_akas = file_name == "title.akas.tsv.gz";
            if !is_basics {
                let basics_done = basics_done.lock().await;
                if !basics_done.0 {
                    let notifier = basics_done.1.clone();
                    drop(basics_done); // or else we hold the lock while waiting
                    tracing::debug!(file = file_name, "waiting for basics to finish");
                    notifier.notified().await;
                    tracing::debug!(file = file_name, "waited for basics");
                }
            }

            if !meta.imported_at.is_some() {
                import_file(&pool, &importer, &mut meta).await?;
                if is_basics || is_akas {
                    needs_search_update.store(true, Ordering::Relaxed);
                }
            }

            if is_basics {
                let mut basics_done = basics_done.lock().await;
                basics_done.0 = true;
                basics_done.1.notify_waiters();
            }

            Ok(())
        }))
    }

    try_join_all(tasks).await?;

    if needs_search_update.load(Ordering::Relaxed) {
        // update the search index
        tracing::info!("rebuilding search index");
        let start = Instant::now();
        let mut tx = pool.begin().await?;

        // Clear the search index
        sqlx::query!("DELETE FROM search_index")
            .execute(&mut *tx)
            .await?;

        // Insert deduplicated titles with priority for primary titles over AKAs
        sqlx::query!(
            "WITH combined_titles AS (
                -- Primary titles (highest priority)
                SELECT 
                    primary_title as text, 
                    1 as is_display, 
                    id as title_id, 
                    0 as ordering,
                    1 as priority
                FROM titles 
                WHERE primary_title IS NOT NULL AND primary_title != ''
                
                UNION ALL
                
                -- AKA titles (lower priority, ordered by their original ordering)
                SELECT 
                    title as text, 
                    0 as is_display, 
                    id as title_id, 
                    ordering,
                    2 as priority
                FROM akas 
                WHERE title IS NOT NULL AND title != ''
            ),
            deduplicated AS (
                SELECT 
                    text, 
                    is_display, 
                    title_id, 
                    ordering,
                    ROW_NUMBER() OVER (
                        PARTITION BY text, title_id 
                        ORDER BY priority ASC, ordering ASC
                    ) as rn
                FROM combined_titles
            )
            INSERT INTO search_index (text, is_display, title_id, ordering)
            SELECT text, is_display, title_id, ordering
            FROM deduplicated
            WHERE rn = 1"
        )
        .execute(&mut *tx)
        .await?;

        tracing::debug!("committing search index changes");
        tx.commit().await?;
        tracing::info!("search index rebuild complete in {:?}", start.elapsed());
    }

    tracing::info!("up to date");
    Ok(())
}
