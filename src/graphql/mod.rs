use std::collections::HashMap;

use crate::id::Id;
use async_graphql::{dataloader::DataLoader, Context, Object, Result};
use tantivy::{
    collector::TopDocs, query::QueryParser, schema::Value, DocId, Index, IndexReader, Score,
    SegmentReader, TantivyDocument,
};
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
        let index = ctx.data::<Index>()?;
        let reader = ctx.data::<IndexReader>()?;

        if let Some(query) = query {
            let searcher = reader.searcher();

            let title_field = index.schema().get_field("title").unwrap();
            let id_field = index.schema().get_field("id").unwrap();

            let query_parser = QueryParser::for_index(index, vec![title_field]);

            let limit = limit.unwrap_or(25);
            let query = query_parser.parse_query(&query).unwrap();
            let top_docs = searcher
                .search(
                    &query,
                    &TopDocs::with_limit(limit).tweak_score(
                        move |segment_reader: &SegmentReader| {
                            let votes_reader = segment_reader
                                .fast_fields()
                                .u64("votes")
                                .unwrap()
                                .first_or_default_col(0);

                            let is_display_reader = segment_reader
                                .fast_fields()
                                .bool("is_display")
                                .unwrap()
                                .first_or_default_col(false);

                            move |doc: DocId, original_score: Score| {
                                let votes: u64 = votes_reader.get_val(doc);
                                let is_display: bool = is_display_reader.get_val(doc);
                                let display_modifier = if is_display { 1.0 } else { -5.0 };
                                original_score * ((votes as f32).log10()) + display_modifier
                            }
                        },
                    ),
                )
                .unwrap();

            for (score, doc_address) in top_docs {
                let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();
                let imdb_id: Id = retrieved_doc
                    .get_first(id_field)
                    .unwrap()
                    .as_u64()
                    .unwrap()
                    .into();

                ids.push(imdb_id);
                scores.insert(imdb_id, score);
            }
        }

        if ids.is_empty() {
            return Err("No IDs to filter for".into());
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
