use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Subscription {
    id: String,
    filters: Vec<Filter>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Filter {
    ids: Option<Vec<String>>,
    authors: Option<Vec<String>>,
    kinds: Option<Vec<i64>>,
    #[serde(rename = "#e")]
    e: Option<Vec<String>>,
    #[serde(rename = "#p")]
    p: Option<Vec<String>>,
    since: Option<i64>,
    until: Option<i64>,
    limit: Option<usize>,
}

impl Subscription {
    pub fn get_id(&self) -> String {
        self.id.clone()
    }
}