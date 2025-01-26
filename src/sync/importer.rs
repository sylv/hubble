use csv_async::StringRecord;
use roaring::RoaringBitmap;
use sqlx::Transaction;

#[derive(PartialEq)]
pub enum ImporterScheduling {
    IsBasics,
    WaitForBasics,
}

#[async_trait::async_trait]
pub trait Importer: Send + Sync {
    fn get_name(&self) -> &str;
    fn get_url(&self) -> &str;
    fn get_bind_count(&self) -> usize;

    fn get_scheduling(&self) -> ImporterScheduling {
        ImporterScheduling::WaitForBasics
    }

    async fn write_batch(
        &self,
        known_ids: &mut RoaringBitmap,
        headers: &StringRecord,
        rows: Vec<StringRecord>,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
    ) -> anyhow::Result<()>;
}
