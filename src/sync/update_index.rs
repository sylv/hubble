use anyhow::Result;
use futures::TryStreamExt;
use sqlx::query;
use tantivy::{
    collector::TopDocs,
    doc,
    query::{BooleanQuery, TermQuery},
    schema::IndexRecordOption,
    Index, Term,
};
use tracing::{debug, info};

pub async fn update_index(index: &Index, pool: &sqlx::SqlitePool) -> Result<()> {
    let reader = index.reader()?;
    let searcher = reader.searcher();

    let id = index.schema().get_field("id")?;
    let votes_field = index.schema().get_field("votes")?;
    let ordering_field = index.schema().get_field("ordering")?;
    let is_display_field = index.schema().get_field("is_display")?;
    let title_field = index.schema().get_field("title")?;

    let mut index_writer = index.writer(100_000_000)?;
    let mut stream = query!(
        "
        SELECT akas.id, akas.ordering, ratings.num_votes, title, titles.primary_title, titles.original_title FROM akas
        LEFT JOIN titles ON titles.id = akas.id
        LEFT JOIN ratings ON ratings.id = akas.id
        WHERE titles.type IN (0, 3, 6) AND ratings.num_votes > 50
        GROUP BY akas.id, title
        "
    )
    .fetch(pool);

    let mut inserted = 0;
    let mut checked = 0;
    let mut last_log = std::time::Instant::now();

    while let Some(row) = stream.try_next().await? {
        let id_term = Term::from_field_u64(id, row.id as u64);
        let ordering_term = Term::from_field_u64(ordering_field, row.ordering as u64);

        let boolean_query = BooleanQuery::new(vec![
            (
                tantivy::query::Occur::Must,
                Box::new(TermQuery::new(id_term, IndexRecordOption::Basic)),
            ),
            (
                tantivy::query::Occur::Must,
                Box::new(TermQuery::new(ordering_term, IndexRecordOption::Basic)),
            ),
        ]);

        if searcher
            .search(&boolean_query, &TopDocs::with_limit(1))?
            .is_empty()
        {
            let is_display_value =
                row.title == row.primary_title || Some(&row.title) == row.original_title.as_ref();

            index_writer.add_document(doc!(
                id => row.id as u64,
                votes_field => row.num_votes as u64,
                ordering_field => row.ordering as u64,
                is_display_field => is_display_value,
                title_field => row.title,
            ))?;

            inserted += 1;
        } else {
            checked += 1
        }

        if last_log.elapsed().as_secs() > 5 {
            last_log = std::time::Instant::now();
            if inserted > 1 || checked > 1 {
                info!("Indexed {} and checked {} documents", inserted, checked);
            }
        }
    }

    debug!("Committing index");
    index_writer.commit()?;
    info!("Indexed {} and checked {} documents", inserted, checked);
    Ok(())
}
