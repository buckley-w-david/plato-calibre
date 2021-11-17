use anyhow::Error;
use chrono::prelude::*;
use reqwest::blocking::Client;
use reqwest::Result as ReqwestResult;
use serde::Deserialize;
use serde_json::json;

use crate::utils::{authors, datetime_format, identifier};
use const_format::concatcp;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const NAME: &'static str = env!("CARGO_PKG_NAME");
const USER_AGENT: &'static str = concatcp!(NAME, " ", VERSION);

pub struct ContentServer {
    client: Client,
    base_url: String,
    username: String,
    password: String,
}

impl ContentServer {
    pub fn new(
        client: Client,
        base_url: String,
        username: String,
        password: String,
    ) -> ContentServer {
        ContentServer {
            client,
            base_url: base_url,
            username: username,
            password: password,
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

        Ok(self
            .client
            .get(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT.to_string())
            .basic_auth(&self.username, Some(&self.password))
            .send()?
            .json()?)
    }

    // TODO: don't return a response, probably take a Write or something and call copy_to
    pub fn epub(&self, book_id: u64, library: &str) -> ReqwestResult<reqwest::blocking::Response> {
        let url = format!("{}/get/EPUB/{}/{}", self.base_url, book_id, library);

        self.client
            .get(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT.to_string())
            .basic_auth(&self.username, Some(&self.password))
            .send()
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

            let response = self
                .content_server
                .client
                .get(&url)
                .header(reqwest::header::USER_AGENT, USER_AGENT.to_string())
                .basic_auth(
                    &self.content_server.username,
                    Some(&self.content_server.password),
                )
                .query(&query)
                .send();

            if let Ok(response) = response {
                if let Ok(category_items) = response.json::<BooksInResposne>() {
                    self.content = Some(category_items.book_ids);
                    self.idx = 1;
                    self.count = category_items.num as usize;
                    self.offset += self.num;

                    if self.count != 0 {
                        result = Some(self.content.as_ref().unwrap()[0]);
                    }
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
pub struct BookMetadata {
    #[serde(with = "authors", rename = "authors")]
    pub author: String,

    pub title: String,

    #[serde(with = "identifier", rename = "identifiers")]
    pub identifier: String,

    #[serde(with = "datetime_format")]
    pub timestamp: DateTime<Utc>,
}
