use crate::{id::Id, sync::importer::Importer};
use anyhow::Result;
use csv_async::StringRecord;
use roaring::RoaringBitmap;
use serde::Deserialize;
use sqlx::{QueryBuilder, Transaction};

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
        known_ids: &mut RoaringBitmap,
        headers: &StringRecord,
        rows: Vec<StringRecord>,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
    ) -> Result<()> {
        let mut qb = QueryBuilder::new("INSERT INTO ratings (id, average_rating, num_votes) ");
        let rows: Vec<RatingsRow> = rows
            .into_iter()
            .filter_map(|row| {
                let row: RatingsRow = row.deserialize(Some(headers)).ok()?;
                if known_ids.contains(row.tconst.get() as u32) {
                    Some(row)
                } else {
                    None
                }
            })
            .collect();

        qb.push_values(rows, |mut qb, row| {
            qb.push_bind(row.tconst.get())
                .push_bind(row.average_rating)
                .push_bind(row.num_votes);
        });

        qb.push(
            " ON CONFLICT(id) DO UPDATE SET
                average_rating = excluded.average_rating,
                num_votes = excluded.num_votes",
        );

        let query = qb.build();
        query.execute(tx.as_mut()).await?;
        Ok(())
    }
}
