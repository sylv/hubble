use crate::id::Id;
use crate::kind::TitleKind;
use crate::sync::importers::Importer;
use crate::sync::nullable::nullable;
use anyhow::Result;
use csv_async::StringRecord;
use serde::Deserialize;
use serde_with::serde_as;
use serde_with::BoolFromInt;
use sqlx::{QueryBuilder, SqlitePool};

#[serde_as]
#[derive(Debug, Deserialize)]
struct BasicsRow {
    tconst: Id,
    #[serde(rename = "titleType")]
    title_type: TitleKind,
    #[serde(rename = "primaryTitle")]
    primary_title: String,
    #[serde(rename = "originalTitle")]
    original_title: String,
    #[serde(rename = "isAdult")]
    #[serde_as(as = "BoolFromInt")]
    is_adult: bool,
    #[serde(rename = "startYear")]
    #[serde(deserialize_with = "nullable")]
    start_year: Option<i32>,
    #[serde(rename = "endYear")]
    #[serde(deserialize_with = "nullable")]
    end_year: Option<i32>,
    #[serde(rename = "runtimeMinutes")]
    #[serde(deserialize_with = "nullable")]
    runtime_minutes: Option<i32>,
    #[serde(deserialize_with = "nullable")]
    genres: Option<String>,
}

pub struct BasicsImporter;

#[async_trait::async_trait]
impl Importer for BasicsImporter {
    fn get_name(&self) -> &str {
        "title.basics.tsv.gz"
    }

    fn get_url(&self) -> &str {
        "https://datasets.imdbws.com/title.basics.tsv.gz"
    }

    fn get_bind_count(&self) -> usize {
        9
    }

    async fn write_batch(
        &self,
        pool: &SqlitePool,
        headers: &StringRecord,
        rows: Vec<StringRecord>,
    ) -> Result<()> {
        let mut qb = QueryBuilder::new("INSERT OR REPLACE INTO titles (id, type, primary_title, original_title, is_adult, start_year, end_year, runtime_minutes, genres) ");

        let rows: Vec<BasicsRow> = rows
            .into_iter()
            .filter_map(|row| row.deserialize(Some(headers)).ok())
            .collect();

        qb.push_values(rows, |mut qb, row| {
            let id = row.tconst.get();
            let original_title = if row.original_title == row.primary_title {
                None
            } else {
                Some(row.original_title)
            };

            let kind = row.title_type as i32;
            qb.push_bind(id)
                .push_bind(kind)
                .push_bind(row.primary_title)
                .push_bind(original_title)
                .push_bind(row.is_adult)
                .push_bind(row.start_year)
                .push_bind(row.end_year)
                .push_bind(row.runtime_minutes)
                .push_bind(row.genres);
        });

        let query = qb.build();
        query.execute(pool).await?;
        Ok(())
    }
}
