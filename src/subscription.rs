//! Subscription and filter parsing
use crate::error::Result;
use crate::event::Event;
use serde::de::Unexpected;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;
use std::collections::HashSet;

/// Subscription identifier and set of request filters
#[derive(Serialize, PartialEq, Eq, Debug, Clone)]
pub struct Subscription {
    pub id: String,
    pub filters: Vec<ReqFilter>,
}

/// Filter for requests
///
/// Corresponds to client-provided subscription request elements. Any
/// element can be present if it should be used in filtering, or
/// absent ([`None`]) if it should be ignored.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ReqFilter {
    /// Event hashes
    pub ids: Option<Vec<String>>,
    /// Event kinds
    pub kinds: Option<Vec<u64>>,
    /// Events published after this time
    pub since: Option<u64>,
    /// Events published before this time
    pub until: Option<u64>,
    /// List of author public keys
    pub authors: Option<Vec<String>>,
    /// Limit number of results
    pub limit: Option<u64>,
    /// Set of tags
    pub tags: Option<HashMap<char, HashSet<String>>>,
    /// Force no matches due to malformed data
    pub force_no_match: bool,
}

impl Serialize for ReqFilter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        if let Some(ids) = &self.ids {
            map.serialize_entry("ids", &ids)?;
        }
        if let Some(kinds) = &self.kinds {
            map.serialize_entry("kinds", &kinds)?;
        }
        if let Some(until) = &self.until {
            map.serialize_entry("until", until)?;
        }
        if let Some(since) = &self.since {
            map.serialize_entry("since", since)?;
        }
        if let Some(limit) = &self.limit {
            map.serialize_entry("limit", limit)?;
        }
        if let Some(authors) = &self.authors {
            map.serialize_entry("authors", &authors)?;
        }
        // serialize tags
        if let Some(tags) = &self.tags {
            for (k, v) in tags {
                let vals: Vec<&String> = v.iter().collect();
                map.serialize_entry(&format!("#{k}"), &vals)?;
            }
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for ReqFilter {
    fn deserialize<D>(deserializer: D) -> Result<ReqFilter, D::Error>
    where
        D: Deserializer<'de>,
    {
        let received: Value = Deserialize::deserialize(deserializer)?;
        let filter = received.as_object().ok_or_else(|| {
            serde::de::Error::invalid_type(
                Unexpected::Other("reqfilter is not an object"),
                &"a json object",
            )
        })?;
        let mut rf = ReqFilter {
            ids: None,
            kinds: None,
            since: None,
            until: None,
            authors: None,
            limit: None,
            tags: None,
            force_no_match: false,
        };
        let empty_string = "".into();
        let mut ts = None;
        // iterate through each key, and assign values that exist
        for (key, val) in filter {
            // ids
            if key == "ids" {
                let raw_ids: Option<Vec<String>> = Deserialize::deserialize(val).ok();
                if let Some(a) = raw_ids.as_ref() {
                    if a.contains(&empty_string) {
                        return Err(serde::de::Error::invalid_type(
                            Unexpected::Other("prefix matches must not be empty strings"),
                            &"a json object",
                        ));
                    }
                }
                rf.ids = raw_ids;
            } else if key == "kinds" {
                rf.kinds = Deserialize::deserialize(val).ok();
            } else if key == "since" {
                rf.since = Deserialize::deserialize(val).ok();
            } else if key == "until" {
                rf.until = Deserialize::deserialize(val).ok();
            } else if key == "limit" {
                rf.limit = Deserialize::deserialize(val).ok();
            } else if key == "authors" {
                let raw_authors: Option<Vec<String>> = Deserialize::deserialize(val).ok();
                if let Some(a) = raw_authors.as_ref() {
                    if a.contains(&empty_string) {
                        return Err(serde::de::Error::invalid_type(
                            Unexpected::Other("prefix matches must not be empty strings"),
                            &"a json object",
                        ));
                    }
                }
                rf.authors = raw_authors;
            } else if key.starts_with('#') && key.len() > 1 && val.is_array() {
                if let Some(tag_search) = tag_search_char_from_filter(key) {
                    if ts.is_none() {
                        // Initialize the tag if necessary
                        ts = Some(HashMap::new());
                    }
                    if let Some(m) = ts.as_mut() {
                        let tag_vals: Option<Vec<String>> = Deserialize::deserialize(val).ok();
                        if let Some(v) = tag_vals {
                            let hs = v.into_iter().collect::<HashSet<_>>();
                            m.insert(tag_search.to_owned(), hs);
                        }
                    };
                } else {
                    // tag search that is multi-character, don't add to subscription
                    rf.force_no_match = true;
                    continue;
                }
            }
        }
        rf.tags = ts;
        Ok(rf)
    }
}

/// Attempt to form a single-char identifier from a tag search filter
fn tag_search_char_from_filter(tagname: &str) -> Option<char> {
    let tagname_nohash = &tagname[1..];
    // We return the tag character if and only if the tagname consists
    // of a single char.
    let mut tagnamechars = tagname_nohash.chars();
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

impl<'de> Deserialize<'de> for Subscription {
    fn deserialize<D>(deserializer: D) -> Result<Subscription, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut v: Value = Deserialize::deserialize(deserializer)?;
        let va = v
            .as_array_mut()
            .ok_or_else(|| serde::de::Error::custom("not array"))?;

        if va.len() < 3 {
            return Err(serde::de::Error::custom("not enough fields"));
        }
        let mut i = va.iter_mut();
        
        let req_cmd_str: serde_json::Value = i.next().unwrap().take();
        let req = req_cmd_str
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("first element of request was not a string"))?;
        if req != "REQ" {
            return Err(serde::de::Error::custom("missing REQ command"));
        }

        let sub_id_str: serde_json::Value = i.next().unwrap().take();
        let sub_id = sub_id_str
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("missing subscription id"))?;

        let mut filters = vec![];
        for fv in i {
            let f: ReqFilter = serde_json::from_value(fv.take())
                .map_err(|_| serde::de::Error::custom("could not parse filter"))?;
            filters.push(f);
        }
        filters.dedup();
        Ok(Subscription {
            id: sub_id.to_owned(),
            filters,
        })
    }
}

impl Subscription {
    #[must_use]
    pub fn get_id(&self) -> String {
        self.id.clone()
    }

    /// Determine if any filter is requesting historical (database)
    /// queries. If every filter has limit:0, we do not need to query the DB.
    #[must_use]
    pub fn needs_historical_events(&self) -> bool {
        self.filters.iter().any(|f| f.limit != Some(0))
    }

    #[must_use]
    pub fn interested_in_event(&self, event: &Event) -> bool {
        for f in &self.filters {
            if f.interested_in_event(event) {
                return true;
            }
        }
        false
    }
}

fn prefix_match(prefixes: &[String], target: &str) -> bool {
    for prefix in prefixes {
        if target.starts_with(prefix) {
            return true;
        }
    }
    false
}

impl ReqFilter {
    fn ids_match(&self, event: &Event) -> bool {
        self.ids
            .as_ref()
            .map_or(true, |vs| prefix_match(vs, &event.id))
    }

    fn authors_match(&self, event: &Event) -> bool {
        self.authors
            .as_ref()
            .map_or(true, |vs| prefix_match(vs, &event.pubkey))
    }

    fn delegated_authors_match(&self, event: &Event) -> bool {
        if let Some(delegated_pubkey) = &event.delegated_by {
            self.authors
                .as_ref()
                .map_or(true, |vs| prefix_match(vs, delegated_pubkey))
        } else {
            false
        }
    }

    fn tag_match(&self, event: &Event) -> bool {
        if let Some(map) = &self.tags {
            for (key, val) in map.iter() {
                let tag_match = event.generic_tag_val_intersect(*key, val);
                if !tag_match {
                    return false;
                }
            }
        }
        true
    }

    fn kind_match(&self, kind: u64) -> bool {
        self.kinds.as_ref().map_or(true, |ks| ks.contains(&kind))
    }

    #[must_use]
    pub fn interested_in_event(&self, event: &Event) -> bool {
        self.ids_match(event)
            && self.since.map_or(true, |t| event.created_at >= t)
            && self.until.map_or(true, |t| event.created_at <= t)
            && self.kind_match(event.kind)
            && (self.authors_match(event) || self.delegated_authors_match(event))
            && self.tag_match(event)
            && !self.force_no_match
    }
}