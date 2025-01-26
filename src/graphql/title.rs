use std::collections::HashMap;
use std::sync::Arc;

use super::episode::Episode;
use crate::{id::Id, kind::TitleKind};
use async_graphql::*;
use dataloader::Loader;
use itertools::Itertools;
use sqlx::{query, query_as, sqlite::SqliteRow, FromRow, SqlitePool};
use sqlx::{QueryBuilder, Row};

#[derive(SimpleObject, Clone)]
#[graphql(complex)]
pub struct Title {
    pub id: Id,
    pub kind: TitleKind,
    pub primary_title: String,
    pub original_title: Option<String>,
    pub is_adult: bool,
    pub start_year: Option<i32>,
    pub end_year: Option<i32>,
    pub runtime_minutes: Option<i32>,
    pub genres: Vec<String>,
}

#[ComplexObject]
impl Title {
    async fn episodes(&self, ctx: &Context<'_>) -> Result<Vec<Episode>> {
        let pool = ctx.data::<SqlitePool>()?;
        let id = self.id.get();
        let episodes = query_as!(
            Episode,
            "SELECT 
            id, parent_id, season_number as \"season_number: _\", episode_number as \"episode_number: _\"
            FROM episodes
            WHERE parent_id = ?",
            id
        )
        .fetch_all(pool)
        .await?;

        Ok(episodes)
    }

    async fn rating(&self, ctx: &Context<'_>) -> Result<Option<Rating>> {
        let pool = ctx.data::<SqlitePool>()?;
        let id = self.id.get();
        let rating = query_as!(
            Rating,
            "SELECT 
            num_votes AS \"num_votes: _\", average_rating AS \"average_rating: _\"
            FROM ratings
            WHERE id = ?",
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(rating)
    }

    async fn akas(&self, ctx: &Context<'_>) -> Result<Vec<Aka>> {
        let pool = ctx.data::<SqlitePool>()?;
        let id = self.id.get();
        let akas = query!(
            "SELECT 
            id, ordering, title, region, language, types, attributes, is_original_title
            FROM akas
            WHERE id = ? AND title != ?",
            id,
            self.primary_title
        )
        .fetch_all(pool)
        .await?;

        Ok(akas
            .into_iter()
            .map(|aka| Aka {
                ordering: aka.ordering,
                title: aka.title,
                region: aka.region,
                language: aka.language,
                attributes: aka
                    .attributes
                    .map(|a| a.split(",").map(|s| s.to_string()).collect())
                    .unwrap_or_default(),
                types: aka
                    .types
                    .map(|t| t.split(",").map(|s| s.to_string()).collect())
                    .unwrap_or_default(),
                is_original_title: aka.is_original_title == 1,
            })
            .collect_vec())
    }
}

impl FromRow<'_, SqliteRow> for Title {
    fn from_row(row: &SqliteRow) -> std::result::Result<Self, sqlx::Error> {
        let id: i64 = row.try_get("id")?;
        let genres: Option<String> = row.try_get("genres")?;
        Ok(Self {
            id: id.into(),
            kind: row.try_get("kind")?,
            primary_title: row.try_get("primary_title")?,
            original_title: row.try_get("original_title")?,
            is_adult: row.try_get("is_adult")?,
            start_year: row.try_get("start_year")?,
            end_year: row.try_get("end_year")?,
            runtime_minutes: row.try_get("runtime_minutes")?,
            genres: genres
                .map(|s| s.split(',').map(|s| s.to_string()).collect())
                .unwrap_or_default(),
        })
    }
}

#[derive(SimpleObject, Clone)]
pub struct Rating {
    pub num_votes: u64,
    pub average_rating: f32,
}

#[derive(SimpleObject, Clone)]
pub struct Aka {
    pub ordering: i64,
    pub title: String,
    pub region: Option<String>,
    pub language: Option<String>,
    pub types: Vec<String>,
    pub attributes: Vec<String>,
    pub is_original_title: bool,
}

#[derive(SimpleObject)]
pub struct TitleWithRank {
    #[graphql(flatten)]
    pub title: Title,
    pub rank: Option<f32>,
}

pub struct TitleLoader {
    pool: SqlitePool,
}

impl TitleLoader {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl Loader<Id> for TitleLoader {
    type Value = Title;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Id]) -> Result<HashMap<Id, Self::Value>, Self::Error> {
        let mut query_builder = QueryBuilder::new("SELECT id, type AS kind, primary_title, original_title, start_year, end_year, is_adult, genres, runtime_minutes FROM titles WHERE id IN (");

        let mut is_first = true;
        for id in keys.iter() {
            if is_first {
                is_first = false;
            } else {
                query_builder.push(", ");
            }

            query_builder.push_bind(id.get());
        }

        query_builder.push(")");
        let query = query_builder.build_query_as::<Title>();
        let titles = query.fetch_all(&self.pool).await?;

        Ok(titles
            .into_iter()
            .map(|title| (title.id.clone(), title))
            .collect())
    }
}
