use super::importer::Importer;

mod akas;
mod basics;
mod episodes;
mod ratings;

pub fn get_importers() -> Vec<Box<dyn Importer>> {
    vec![
        Box::new(basics::BasicsImporter),
        Box::new(akas::AkasImporter),
        Box::new(episodes::EpisodesImporter),
        Box::new(ratings::RatingsImporter),
    ]
}
