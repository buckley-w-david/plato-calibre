use anyhow::{format_err, Context, Error};
use chrono::prelude::*;
use chrono::Local;
use reqwest::blocking::Client;
use serde_json::{json, Value as JsonValue};
use serde::{Deserialize, Serialize};

use const_format::concatcp;
use crate::settings::Settings;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const NAME: &'static str = env!("CARGO_PKG_NAME");
const USER_AGENT: &'static str = concatcp!(NAME, " ", VERSION);

pub struct Counter<'a> {
    num: u64,
    offset: u64,
    idx: usize,
    count: usize,
    content: Option<Vec<u64>>,
    client: &'a Client,
    settings: &'a Settings,
}

impl<'a> Counter<'a> {
    fn new(client: &'a Client, settings: &'a Settings) -> Counter<'a> {
        Counter { 
            num: 100, 
            offset: 0, 
            idx: 0,
            count: 0,
            content: None,
            client,
            settings,
        }
    }
}

pub fn books_in<'a>(client: &'a Client, settings: &'a Settings) -> Counter<'a> {
    Counter::new(client, settings)
}

impl Iterator for Counter<'_> {
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
                self.settings.base_url, self.settings.category, self.settings.item, self.settings.library
            );
            let category_items: JsonValue = self.client
                .get(&url)
                .header(reqwest::header::USER_AGENT, USER_AGENT.to_string())
                .basic_auth(&self.settings.username, Some(&self.settings.password))
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


//         let category_items: JsonValue = client
//             .get(&url)
//             .header(reqwest::header::USER_AGENT, USER_AGENT.to_string())
//             .basic_auth(&settings.username, Some(&settings.password))
//             .query(&query)
//             .send()?
//             .json()?;

//         if category_items
//             .get("num")
//             .and_then(JsonValue::as_u64)
//             .unwrap()
//             == 0
//         {
//             break;
//         } else {
//             if let Some(book_ids) = category_items.get("book_ids").and_then(JsonValue::as_array) {
//                 for id in book_ids {
//                     if sigterm.load(Ordering::Relaxed) {
//                         break;
//                     }

//                     let url = format!(
//                         "{}/ajax/book/{}/{}",
//                         &settings.base_url, id, &settings.library
//                     );

//                     let metadata: JsonValue = client
//                         .get(&url)
//                         .header(reqwest::header::USER_AGENT, USER_AGENT.to_string())
//                         .basic_auth(&settings.username, Some(&settings.password))
//                         .query(&query)
//                         .send()?
//                         .json()?;

//                     let title = metadata
//                         .get("title")
//                         .and_then(JsonValue::as_str)
//                         .unwrap_or_default();

//                     let author = metadata
//                         .get("authors")
//                         .and_then(JsonValue::as_array)
//                         .unwrap()
//                         .iter()
//                         .map(|v| v.as_str().unwrap())
//                         .collect::<Vec<&str>>()
//                         .join(", ");

//                     let timestamp = metadata
//                         .get("timestamp")
//                         .and_then(JsonValue::as_str)
//                         .unwrap_or_default()
//                         .parse::<DateTime<Utc>>()
//                         .unwrap_or(Utc::now());

//                     let url_id = metadata
//                         .pointer("/identifiers/url")
//                         .and_then(JsonValue::as_str)
//                         .unwrap_or_default();
//                     let hash_id = fxhash::hash64(url_id).to_string();

//                     if let Some(Response::Search(event)) = (Event::Search{ path: &save_path, query: format!("'i ^{}$", hash_id)}).send() {
//                         if let Some(results) = event.get("results").and_then(JsonValue::as_array) {
//                             let info = results.first();
//                             if let Some(Some(existing_timestamp)) = info.map(|v| v.get("added").and_then(JsonValue::as_str)) {
//                                 if let Ok(tt) = Utc.datetime_from_str(existing_timestamp, "%Y-%m-%d %H:%M:%S") {
//                                     if (tt - timestamp).num_seconds() == 0 {
//                                         // Man this is ugly
//                                         // If the added time close enough, don't sync
//                                         continue;
//                                     }
//                                 }
//                             }
//                         }
//                     }

//                     let epub_path = save_path.join(&format!("{}.epub", hash_id));
//                     let exists = epub_path.exists();
//                     let mut file = File::create(&epub_path)?;
//                     let url = format!(
//                         "{}/get/EPUB/{}/{}",
//                         &settings.base_url, id, &settings.library
//                     );

//                     let response = client
//                         .get(&url)
//                         .header(reqwest::header::USER_AGENT, USER_AGENT.to_string())
//                         .basic_auth(&settings.username, Some(&settings.password))
//                         .send()
//                         .and_then(|mut body| body.copy_to(&mut file));

//                     if let Err(err) = response {
//                         eprintln!("Can't download {}: {:#}.", id, err);
//                         fs::remove_file(epub_path).ok();
//                         continue;
//                     }

//                     if let Ok(path) = epub_path.strip_prefix(&library_path) {
//                         let file_info = json!({
//                             "path": path,
//                             "kind": "epub",
//                             "size": file.metadata().ok()
//                                         .map_or(0, |m| m.len()),
//                         });

//                         let info = json!({
//                             "title": title,
//                             "author": author,
//                             "identifier": hash_id,
//                             "file": file_info,
//                             "added": timestamp.with_timezone(&Local)
//                                                .format("%Y-%m-%d %H:%M:%S")
//                                                .to_string(),
//                         });

//                         let event = if !exists {
//                             Event::AddDocument(&info)
//                         } else {
//                             Event::UpdateDocument{path: &path, info: &info}
//                         };
//                         event.send();
//                     }
//                 }
//             }
//             offset += num;
//             query["offset"] = JsonValue::from(offset);
//         }
//     }
//     Event::Notify("Finished syncing books!").send();

//     Ok(())
// }
