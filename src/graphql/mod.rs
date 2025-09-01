use std::collections::HashMap;

use crate::id::Id;
use async_graphql::{dataloader::DataLoader, Context, Object, Result};
use sqlx::SqlitePool;
use title::{Title, TitleLoader, TitleWithRank};

mod episode;
pub mod title;

pub struct Query;

#[Object]
impl Query {
    async fn title(&self, ctx: &Context<'_>, id: Id) -> Result<Option<Title>> {
        let loader = ctx.data::<DataLoader<TitleLoader>>()?;
        let title = loader.load_one(id).await?;
        Ok(title)
    }

    async fn titles(
        &self,
        ctx: &Context<'_>,
        query: Option<String>,
        ids: Option<Vec<Id>>,
        limit: Option<usize>,
    ) -> Result<Vec<TitleWithRank>> {
        let mut ids = ids.unwrap_or_default();
        let mut scores = HashMap::new();
        let pool = ctx.data::<SqlitePool>()?;

        if let Some(query) = query {
            let limit = limit.unwrap_or(25) as i64;
            let escaped_query = query.replace(":", "");
            let search_results = sqlx::query!(
                r#"
                SELECT 
                    si.title_id AS "title_id: i64",
                    -bm25(search_index)
                    +
                    (
                        CASE
                            WHEN r.num_votes < 10 THEN 1.0
                            WHEN r.num_votes < 100 THEN 1.5
                            WHEN r.num_votes < 1000 THEN 2.0
                            WHEN r.num_votes < 10000 THEN 2.5
                            ELSE 2.5
                        END
                    )
                    +
                    (
                        CASE 
                            WHEN si.is_display = 1 THEN 1.0 
                            ELSE -5.0 
                        END
                    ) AS final_score
                FROM search_index si
                LEFT JOIN ratings r ON r.id = si.title_id
                LEFT JOIN titles t ON t.id = si.title_id
                WHERE 
                    text MATCH ?
                    AND r.id IS NOT NULL 
                    AND t.type IN (0, 1, 3, 4, 6, 10)
                ORDER BY final_score DESC
                LIMIT ?
                "#,
                escaped_query,
                limit
            )
            .fetch_all(pool)
            .await?;

            for result in search_results {
                let title_id: Id = result.title_id.expect("missing title_id").into();
                ids.push(title_id);
                scores.insert(title_id, result.final_score as f32);
            }
        } else {
            if ids.is_empty() {
                return Err("'query' or 'ids' is required'".into());
            }
        }

        let loader = ctx.data::<DataLoader<TitleLoader>>()?;
        let titles = loader.load_many(ids).await?;

        let mut titles = titles
            .into_iter()
            .map(|(_, title)| {
                let id = title.id;
                TitleWithRank {
                    rank: scores.get(&id).copied(),
                    title,
                }
            })
            .collect::<Vec<_>>();

        titles.sort_by(|a, b| {
            let a_score = a.rank.unwrap_or(0.0);
            let b_score = b.rank.unwrap_or(0.0);
            b_score.partial_cmp(&a_score).unwrap()
        });

        Ok(titles)
    }
}
