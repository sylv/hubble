use anyhow::Result;
use std::path::PathBuf;
use tantivy::{
    schema::{Schema, FAST, INDEXED, STORED, TEXT},
    Index,
};

fn get_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    schema_builder.add_u64_field("id", FAST | INDEXED | STORED);
    schema_builder.add_u64_field("ordering", FAST | INDEXED);
    schema_builder.add_u64_field("votes", FAST);
    // whether title is primary or original
    schema_builder.add_bool_field("is_display", FAST);
    schema_builder.add_text_field("title", TEXT);

    schema_builder.build()
}

pub fn get_index(data_dir: &PathBuf) -> Result<Index> {
    let index_path = data_dir.join("index");
    // ensure index dir exists
    std::fs::create_dir_all(&index_path).unwrap();
    let schema = get_schema();
    let dir = tantivy::directory::MmapDirectory::open(&index_path)?;
    let index = Index::open_or_create(dir, schema.clone())?;
    Ok(index)
}
