use crate::id::Id;
use crate::sync::importers::Importer;
use crate::sync::nullable::nullable;
use anyhow::Result;
use csv_async::StringRecord;
use serde::Deserialize;
use serde_with::serde_as;
use serde_with::BoolFromInt;
use sqlx::QueryBuilder;
use sqlx::SqlitePool;

#[serde_as]
#[derive(Debug, Deserialize)]
struct AkasRow {
    #[serde(rename = "titleId")]
    title_id: Id,
    ordering: i32,
    title: String,
    #[serde(deserialize_with = "nullable")]
    region: Option<String>,
    #[serde(deserialize_with = "nullable")]
    language: Option<String>,
    #[serde(deserialize_with = "nullable")]
    types: Option<String>,
    #[serde(deserialize_with = "nullable")]
    attributes: Option<String>,
    #[serde(rename = "isOriginalTitle")]
    #[serde_as(as = "BoolFromInt")]
    is_original_title: bool,
}

pub struct AkasImporter;

#[async_trait::async_trait]
impl Importer for AkasImporter {
    fn get_name(&self) -> &str {
        "title.akas.tsv.gz"
    }

    fn get_url(&self) -> &str {
        "https://datasets.imdbws.com/title.akas.tsv.gz"
    }

    fn get_bind_count(&self) -> usize {
        8
    }

    async fn write_batch(
        &self,
        pool: &SqlitePool,
        headers: &StringRecord,
        rows: Vec<StringRecord>,
    ) -> Result<()> {
        let mut qb = QueryBuilder::new(
            "INSERT OR REPLACE INTO akas (id, ordering, title, region, language, types, attributes, is_original_title) "
        );

        let rows: Vec<AkasRow> = rows
            .into_iter()
            .filter_map(|row| row.deserialize(Some(headers)).ok())
            .collect();

        qb.push_values(rows, |mut qb, row| {
            qb.push_bind(row.title_id.get())
                .push_bind(row.ordering)
                .push_bind(row.title)
                .push_bind(row.region)
                .push_bind(row.language)
                .push_bind(row.types)
                .push_bind(row.attributes)
                .push_bind(row.is_original_title);
        });

        let query = qb.build();
        query.execute(pool).await?;
        Ok(())
    }
}
