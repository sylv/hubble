use crate::sync::nullable::nullable;
use crate::{id::Id, sync::importers::Importer};
use anyhow::Result;
use csv_async::StringRecord;
use serde::Deserialize;
use sqlx::{QueryBuilder, SqlitePool};

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
        pool: &SqlitePool,
        headers: &StringRecord,
        rows: Vec<StringRecord>,
    ) -> Result<()> {
        let mut qb = QueryBuilder::new(
            "INSERT OR REPLACE INTO episodes (id, parent_id, season_number, episode_number) ",
        );
        let rows: Vec<EpisodesRow> = rows
            .into_iter()
            .filter_map(|row| row.deserialize(Some(headers)).ok())
            // filter out rows with null season or episode numbers
            .filter(|row: &EpisodesRow| row.season_number.is_some() && row.episode_number.is_some())
            .collect();

        qb.push_values(rows, |mut qb, row| {
            qb.push_bind(row.tconst.get())
                .push_bind(row.parent_tconst.get())
                .push_bind(row.season_number)
                .push_bind(row.episode_number);
        });

        let query = qb.build();
        query.execute(pool).await?;
        Ok(())
    }
}
