use anyhow::Error;
use chrono::prelude::*;
use reqwest::blocking::Client;
use reqwest::Result as ReqwestResult;
use serde_json::{json, Value as JsonValue};
use serde::Deserialize;

use const_format::concatcp;
use crate::settings::Settings;
use crate::utils::{datetime_format, authors, identifier};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const NAME: &'static str = env!("CARGO_PKG_NAME");
const USER_AGENT: &'static str = concatcp!(NAME, " ", VERSION);

pub struct ContentServer<'a> {
    client: Client,
    base_url: &'a str,
    username: &'a str,
    password: &'a str,
}

impl<'a> ContentServer<'a> {
    pub fn new(client: Client, settings: &'a Settings) -> ContentServer<'a> {
        ContentServer {
            client,
            base_url: &settings.base_url,
            username: &settings.username,
            password: &settings.password,
        }
    }

    pub fn books_in(&'a self, category: u64, item: u64, library: &'a str) -> BooksIn<'a> {
        BooksIn::new(self, category, item, library)
    }

    pub fn metadata(&self, book_id: u64, library: &str) -> Result<BookMetadata, Error> {
        let url = format!(
            "{}/ajax/book/{}/{}",
            self.base_url, book_id, library
        );

        Ok(
            self.client
                .get(&url)
                .header(reqwest::header::USER_AGENT, USER_AGENT.to_string())
                .basic_auth(self.username, Some(self.password))
                .send()?
                .json()?
        )
    }

    // TODO: don't return a response, probably take a Write or something and call copy_to
    pub fn epub(&self, book_id: u64, library: &'a str) -> ReqwestResult<reqwest::blocking::Response> {
        let url = format!(
            "{}/get/EPUB/{}/{}",
            self.base_url, book_id, library
        );

        self.client
            .get(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT.to_string())
            .basic_auth(self.username, Some(self.password))
            .send()
    }
}

pub struct BooksIn<'a> {
    num: u64,
    offset: u64,
    idx: usize,
    count: usize,
    content: Option<Vec<u64>>,
    category: u64, item: u64, library: &'a str,
    content_server: &'a ContentServer<'a>,
}

impl<'a> BooksIn<'a> {
    fn new(content_server: &'a ContentServer, category: u64, item: u64, library: &'a str) -> BooksIn<'a> {
        BooksIn { 
            num: 100, 
            offset: 0, 
            idx: 0,
            count: 0,
            content: None,
            category, item, library, content_server,
        }
    }
}

impl Iterator for BooksIn<'_> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        let mut result: Option<Self::Item> = None;
        if self.idx != self.count {
            if let Some(items) = &self.content {
                result = Some(items[self.idx]);
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

            let category_items: JsonValue = self.content_server.client
                .get(&url)
                .header(reqwest::header::USER_AGENT, USER_AGENT.to_string())
                .basic_auth(&self.content_server.username, Some(&self.content_server.password))
                .query(&query)
                .send().unwrap()
                .json().unwrap();

            if let Some(book_ids) = category_items.get("book_ids").and_then(JsonValue::as_array) {
                let ids = book_ids.iter().map(JsonValue::as_u64).collect();
                self.content = ids;
                self.idx = 1;
                self.count = category_items
                    .get("num")
                    .and_then(JsonValue::as_u64)
                    .unwrap_or_default() as usize;
                self.offset += self.num;

                if self.count != 0 {
                    result = Some(self.content.as_ref().unwrap()[0]);
                    self.idx = 1;
                }
            }
        }
        result
    }
}

#[derive(Debug, Deserialize)]
pub struct BookMetadata {
    #[serde(with = "authors", rename = "authors")]
    pub author: String,

    pub title: String,

    #[serde(with = "identifier", rename = "identifiers")]
    pub identifier: String,

    #[serde(with = "datetime_format")]
    pub timestamp: DateTime<Utc>,
}
