use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Close {
    pub id: String,
}