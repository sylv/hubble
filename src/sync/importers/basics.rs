use crate::id::Id;
use crate::kind::TitleKind;
use crate::sync::importer::Importer;
use crate::sync::importer::ImporterScheduling;
use crate::sync::nullable::nullable;
use anyhow::Result;
use csv_async::StringRecord;
use roaring::RoaringBitmap;
use serde::Deserialize;
use serde_with::serde_as;
use serde_with::BoolFromInt;
use sqlx::{QueryBuilder, Transaction};

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

    fn get_scheduling(&self) -> ImporterScheduling {
        ImporterScheduling::IsBasics
    }

    async fn write_batch(
        &self,
        known_ids: &mut RoaringBitmap,
        headers: &StringRecord,
        rows: Vec<StringRecord>,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
    ) -> Result<()> {
        let mut qb = QueryBuilder::new("INSERT INTO titles (id, type, primary_title, original_title, is_adult, start_year, end_year, runtime_minutes, genres) ");

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

            known_ids.insert(id);

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

        qb.push(
            " ON CONFLICT(id) DO UPDATE SET
                type = excluded.type,
                primary_title = excluded.primary_title,
                original_title = excluded.original_title,
                is_adult = excluded.is_adult,
                start_year = excluded.start_year,
                end_year = excluded.end_year,
                runtime_minutes = excluded.runtime_minutes,
                genres = excluded.genres",
        );

        let query = qb.build();
        query.execute(tx.as_mut()).await?;
        Ok(())
    }
}
