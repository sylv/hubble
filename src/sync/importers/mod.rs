use csv_async::StringRecord;
use sqlx::SqlitePool;

mod akas;
mod basics;
mod episodes;
mod ratings;

#[async_trait::async_trait]
pub trait Importer: Send + Sync {
    fn get_name(&self) -> &str;
    fn get_url(&self) -> &str;
    fn get_bind_count(&self) -> usize;

    async fn write_batch(
        &self,
        pool: &SqlitePool,
        headers: &StringRecord,
        rows: Vec<StringRecord>,
    ) -> anyhow::Result<()>;
}

pub fn get_importers() -> Vec<Box<dyn Importer>> {
    vec![
        Box::new(basics::BasicsImporter),
        Box::new(akas::AkasImporter),
        Box::new(episodes::EpisodesImporter),
        Box::new(ratings::RatingsImporter),
    ]
}
