use serde::Deserialize;

pub fn nullable<'de, D, T, E>(de: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    Option<T>: serde::Deserialize<'de>,
    T: std::str::FromStr<Err = E>,
    E: std::error::Error,
{
    use serde::de::Error;

    let val = String::deserialize(de)?;
    if val.is_empty() || val == "\\N" {
        Ok(None)
    } else {
        val.parse()
            .map(Some)
            .map_err(|e: E| D::Error::custom(e.to_string()))
    }
}
