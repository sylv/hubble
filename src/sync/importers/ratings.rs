use crate::{id::Id, sync::importers::Importer};
use anyhow::Result;
use csv_async::StringRecord;
use serde::Deserialize;
use sqlx::{QueryBuilder, SqlitePool};

#[derive(Debug, Deserialize)]
struct RatingsRow {
    tconst: Id,
    #[serde(rename = "averageRating")]
    average_rating: f32,
    #[serde(rename = "numVotes")]
    num_votes: i32,
}

pub struct RatingsImporter;

#[async_trait::async_trait]
impl Importer for RatingsImporter {
    fn get_name(&self) -> &str {
        "title.ratings.tsv.gz"
    }

    fn get_url(&self) -> &str {
        "https://datasets.imdbws.com/title.ratings.tsv.gz"
    }

    fn get_bind_count(&self) -> usize {
        3
    }

    async fn write_batch(
        &self,
        pool: &SqlitePool,
        headers: &StringRecord,
        rows: Vec<StringRecord>,
    ) -> Result<()> {
        let mut qb =
            QueryBuilder::new("INSERT OR REPLACE INTO ratings (id, average_rating, num_votes) ");
        let rows: Vec<RatingsRow> = rows
            .into_iter()
            .filter_map(|row| row.deserialize(Some(headers)).ok())
            .collect();

        qb.push_values(rows, |mut qb, row| {
            qb.push_bind(row.tconst.get())
                .push_bind(row.average_rating)
                .push_bind(row.num_votes);
        });

        let query = qb.build();
        query.execute(pool).await?;
        Ok(())
    }
}
