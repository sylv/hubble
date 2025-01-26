use super::title::{Title, TitleLoader};
use crate::id::Id;
use async_graphql::*;
use dataloader::DataLoader;

#[derive(SimpleObject)]
#[graphql(complex)]
pub struct Episode {
    pub id: Id,
    pub parent_id: Id,
    pub season_number: i32,
    pub episode_number: i32,
}

#[ComplexObject]
impl Episode {
    async fn title(&self, ctx: &Context<'_>) -> Result<Title> {
        let loader = ctx.data::<DataLoader<TitleLoader>>()?;
        let title = loader.load_one(self.id).await?;
        Ok(title.ok_or_else(|| Error::new("Title for episode not found"))?)
    }
}
