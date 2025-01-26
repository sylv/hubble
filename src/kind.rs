use async_graphql::Enum;
use serde::Deserialize;

#[derive(Enum, Copy, Clone, Eq, PartialEq, Deserialize, Debug, sqlx::Type)]
#[repr(i64)]
pub enum TitleKind {
    #[serde(rename = "movie")]
    Movie = 0,
    #[serde(rename = "short")]
    Short = 1,
    #[serde(rename = "tvEpisode")]
    TvEpisode = 2,
    #[serde(rename = "tvMiniSeries")]
    TvMiniSeries = 3,
    #[serde(rename = "tvMovie")]
    TvMovie = 4,
    #[serde(rename = "tvPilot")]
    TvPilot = 5,
    #[serde(rename = "tvSeries")]
    TvSeries = 6,
    #[serde(rename = "tvShort")]
    TvShort = 7,
    #[serde(rename = "tvSpecial")]
    TvSpecial = 8,
    #[serde(rename = "video")]
    Video = 9,
    #[serde(rename = "videoGame")]
    VideoGame = 10,
}

impl From<i64> for TitleKind {
    fn from(kind: i64) -> Self {
        match kind {
            0 => TitleKind::Movie,
            1 => TitleKind::Short,
            2 => TitleKind::TvEpisode,
            3 => TitleKind::TvMiniSeries,
            4 => TitleKind::TvMovie,
            5 => TitleKind::TvPilot,
            6 => TitleKind::TvSeries,
            7 => TitleKind::TvShort,
            8 => TitleKind::TvSpecial,
            9 => TitleKind::Video,
            10 => TitleKind::VideoGame,
            _ => panic!("Invalid title kind"),
        }
    }
}
