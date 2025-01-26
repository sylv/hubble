use crate::sync::nullable::nullable;
use crate::{id::Id, sync::importer::Importer};
use anyhow::Result;
use csv_async::StringRecord;
use roaring::RoaringBitmap;
use serde::Deserialize;
use sqlx::{QueryBuilder, Transaction};

#[derive(Debug, Deserialize)]
struct EpisodesRow {
    tconst: Id,
    #[serde(rename = "parentTconst")]
    parent_tconst: Id,
    #[serde(rename = "seasonNumber")]
    #[serde(deserialize_with = "nullable")]
    season_number: Option<i32>,
    #[serde(rename = "episodeNumber")]
    #[serde(deserialize_with = "nullable")]
    episode_number: Option<i32>,
}

pub struct EpisodesImporter;

#[async_trait::async_trait]
impl Importer for EpisodesImporter {
    fn get_name(&self) -> &str {
        "title.episode.tsv.gz"
    }

    fn get_url(&self) -> &str {
        "https://datasets.imdbws.com/title.episode.tsv.gz"
    }

    fn get_bind_count(&self) -> usize {
        4
    }

    async fn write_batch(
        &self,
        known_ids: &mut RoaringBitmap,
        headers: &StringRecord,
        rows: Vec<StringRecord>,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
    ) -> Result<()> {
        let mut qb = QueryBuilder::new(
            "INSERT INTO episodes (id, parent_id, season_number, episode_number) ",
        );
        let rows: Vec<EpisodesRow> = rows
            .into_iter()
            .filter_map(|row| {
                let row: EpisodesRow = row.deserialize(Some(headers)).ok()?;
                if known_ids.contains(row.tconst.get() as u32) {
                    Some(row)
                } else {
                    None
                }
            })
            // Filter out rows with null season or episode numbers
            .filter(|row: &EpisodesRow| row.season_number.is_some() && row.episode_number.is_some())
            .collect();

        qb.push_values(rows, |mut qb, row| {
            qb.push_bind(row.tconst.get())
                .push_bind(row.parent_tconst.get())
                .push_bind(row.season_number)
                .push_bind(row.episode_number);
        });

        qb.push(
            " ON CONFLICT(id) DO UPDATE SET
                parent_id = excluded.parent_id,
                season_number = excluded.season_number,
                episode_number = excluded.episode_number",
        );

        let query = qb.build();
        query.execute(tx.as_mut()).await?;
        Ok(())
    }
}
