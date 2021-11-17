use anyhow::Error;
use chrono::prelude::*;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

use const_format::concatcp;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const NAME: &'static str = env!("CARGO_PKG_NAME");
const USER_AGENT: &'static str = concatcp!(NAME, " ", VERSION);

pub struct ContentServer {
    client: Client,
    base_url: String,
    username: Option<String>,
    password: Option<String>,
}

impl ContentServer {
    pub fn new(
        client: Client,
        base_url: String,
        username: Option<String>,
        password: Option<String>,
    ) -> ContentServer {
        ContentServer {
            client,
            base_url,
            username,
            password,
        }
    }

    pub fn books_in<'a>(&'a self, category: u64, item: u64, library: &'a str) -> BooksIn<'a> {
        BooksIn {
            num: 100,
            offset: 0,
            idx: 0,
            count: 0,
            content: None,
            category,
            item,
            library,
            content_server: self,
        }
    }

    pub fn metadata(&self, book_id: u64, library: &str) -> Result<BookMetadata, Error> {
        let url = format!("{}/ajax/book/{}/{}", self.base_url, book_id, library);

        let mut request_builder = self
            .client
            .get(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT.to_string());

        if let Some(username) = &self.username {
            request_builder = request_builder.basic_auth(username, self.password.as_ref())
        }

        Ok(request_builder.send()?.json()?)
    }

    pub fn epub<W: ?Sized>(
        &self,
        book_id: u64,
        library: &str,
        w: &mut W,
    ) -> Result<u64, reqwest::Error>
    where
        W: std::io::Write,
    {
        let url = format!("{}/get/EPUB/{}/{}", self.base_url, book_id, library);
        let mut request_builder = self
            .client
            .get(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT.to_string());

        if let Some(username) = &self.username {
            request_builder = request_builder.basic_auth(username, self.password.as_ref())
        }

        request_builder.send().and_then(|mut body| body.copy_to(w))
    }
}

pub struct BooksIn<'a> {
    num: u64,
    offset: u64,
    idx: usize,
    count: usize,
    content: Option<Vec<u64>>,
    category: u64,
    item: u64,
    library: &'a str,
    content_server: &'a ContentServer,
}

impl Iterator for BooksIn<'_> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        let mut result: Option<Self::Item> = None;
        if self.idx != self.count {
            if let Some(items) = &self.content {
                result = Some(items[self.idx]);
                self.idx += 1;
            }
        } else {
            let query = json!({
                "offset": self.offset,
                "num": self.num,
            });

            let url = format!(
                "{}/ajax/books_in/{}/{}/{}",
                self.content_server.base_url, self.category, self.item, self.library
            );

            let mut request_builder = self
                .content_server
                .client
                .get(&url)
                .header(reqwest::header::USER_AGENT, USER_AGENT.to_string());

            if let Some(username) = &self.content_server.username {
                request_builder =
                    request_builder.basic_auth(username, self.content_server.password.as_ref())
            }

            let response = request_builder.query(&query).send();

            if let Ok(response) = response {
                if let Ok(category_items) = response.json::<BooksInResposne>() {
                    if category_items.num != 0 {
                        result = Some(category_items.book_ids[0]);
                    }

                    self.content = Some(category_items.book_ids);
                    self.idx = 1;
                    self.count = category_items.num as usize;
                    self.offset += self.num;
                }
            }
        }
        result
    }
}

#[derive(Debug, Deserialize)]
struct BooksInResposne {
    book_ids: Vec<u64>,
    num: u64,
}

#[derive(Debug, Deserialize)]
pub struct Identifier {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct BookMetadata {
    #[serde(with = "authors", rename = "authors")]
    pub author: String,

    pub title: String,

    pub identifiers: HashMap<String, String>,

    #[serde(with = "datetime_format")]
    pub timestamp: DateTime<Utc>,
}

mod datetime_format {
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<DateTime<Utc>>().map_err(serde::de::Error::custom)
    }
}

mod authors {
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Vec::<String>::deserialize(deserializer)?.join(", "))
    }
}
