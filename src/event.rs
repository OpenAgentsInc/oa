//! Event parsing and validation
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Event {
    pub id: String,
    pub pubkey: String,
    #[serde(skip)]
    pub delegated_by: Option<String>,
    pub created_at: u64,
    pub kind: u64,
    #[serde(deserialize_with = "tag_from_string")]
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
    #[serde(skip)]
    pub tagidx: Option<HashMap<char, HashSet<String>>>,
}

/// Simple tag type for array of array of strings.
type Tag = Vec<Vec<String>>;

/// Deserializer that ensures we always have a [`Tag`].
fn tag_from_string<'de, D>(deserializer: D) -> Result<Tag, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

impl Event {
    pub fn validate(&self) -> Result<()> {
        // For now, just return Ok since we're using SQLx
        Ok(())
    }

    pub fn build_index(&mut self) {
        // if there are no tags; just leave the index as None
        if self.tags.is_empty() {
            return;
        }
        // otherwise, build an index
        let mut idx: HashMap<char, HashSet<String>> = HashMap::new();
        // iterate over tags that have at least 2 elements
        for t in self.tags.iter().filter(|x| x.len() > 1) {
            let tagname = t.first().unwrap();
            let tagnamechar_opt = single_char_tagname(tagname);
            if tagnamechar_opt.is_none() {
                continue;
            }
            let tagnamechar = tagnamechar_opt.unwrap();
            let tagval = t.get(1).unwrap();
            // ensure a vector exists for this tag
            idx.entry(tagnamechar).or_default();
            // get the tag vec and insert entry
            let idx_tag_vec = idx.get_mut(&tagnamechar).expect("could not get tag vector");
            idx_tag_vec.insert(tagval.clone());
        }
        // save the tag structure
        self.tagidx = Some(idx);
    }

    pub fn generic_tag_val_intersect(&self, tagname: char, check: &HashSet<String>) -> bool {
        match &self.tagidx {
            Some(idx) => match idx.get(&tagname) {
                Some(valset) => {
                    let common = valset.intersection(check);
                    common.count() > 0
                }
                None => false,
            },
            None => false,
        }
    }
}

/// Attempt to form a single-char tag name.
#[must_use]
pub fn single_char_tagname(tagname: &str) -> Option<char> {
    // We return the tag character if and only if the tagname consists
    // of a single char.
    let mut tagnamechars = tagname.chars();
    let firstchar = tagnamechars.next();
    match firstchar {
        Some(_) => {
            // check second char
            if tagnamechars.next().is_none() {
                firstchar
            } else {
                None
            }
        }
        None => None,
    }
}