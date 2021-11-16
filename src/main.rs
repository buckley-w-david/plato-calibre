use anyhow::{format_err, Context, Error};
use chrono::prelude::*;
use chrono::Local;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::env;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use const_format::concatcp;

const SETTINGS_PATH: &str = "Settings.toml";

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const NAME: &'static str = env!("CARGO_PKG_NAME");
const USER_AGENT: &'static str = concatcp!(NAME, " ", VERSION);
// const USER_AGENT: &'static str = "Mozilla/5.0 (X11; Linux x86_64; rv:94.0) Gecko/20100101 Firefox/94.0";

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    base_url: String,
    username: String,
    password: String,
    category: u64,
    item: u64,
    library: String,
}

fn load_toml<T, P: AsRef<Path>>(path: P) -> Result<T, Error>
where
    for<'a> T: Deserialize<'a>,
{
    let s = fs::read_to_string(path.as_ref())
        .with_context(|| format!("can't read file {}", path.as_ref().display()))?;
    toml::from_str(&s)
        .with_context(|| format!("can't parse TOML content from {}", path.as_ref().display()))
        .map_err(Into::into)
}

fn main() -> Result<(), Error> {
    let mut args = env::args().skip(1);
    let library_path = PathBuf::from(
        args.next()
            .ok_or_else(|| format_err!("missing argument: library path"))?,
    );
    let save_path = PathBuf::from(
        args.next()
            .ok_or_else(|| format_err!("missing argument: save path"))?,
    );
    let wifi = args
        .next()
        .ok_or_else(|| format_err!("missing argument: wifi status"))
        .and_then(|v| v.parse::<bool>().map_err(Into::into))?;
    let online = args
        .next()
        .ok_or_else(|| format_err!("missing argument: online status"))
        .and_then(|v| v.parse::<bool>().map_err(Into::into))?;

    let settings = load_toml::<Settings, _>(SETTINGS_PATH)
        .with_context(|| format!("can't load settings from {}", SETTINGS_PATH))?;

    if !online {
        if !wifi {
            let event = json!({
                "type": "notify",
                "message": "Establishing a network connection.",
            });
            println!("{}", event);
            let event = json!({
                "type": "setWifi",
                "enable": true,
            });
            println!("{}", event);
        } else {
            let event = json!({
                "type": "notify",
                "message": "Waiting for the network to come up.",
            });
            println!("{}", event);
        }
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
    }

    if !save_path.exists() {
        fs::create_dir(&save_path)?;
    }

    let client = Client::new();

    let sigterm = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&sigterm))?;

    let url = format!(
        "{}/ajax/books_in/{}/{}/{}",
        &settings.base_url, &settings.category, &settings.item, &settings.library
    );
    let num = 100;
    let mut offset = 0;
    let mut query = json!({
        "offset": offset,
        "num": num,
    });

    loop {
        if sigterm.load(Ordering::Relaxed) {
            break;
        }

        let category_items: JsonValue = client
            .get(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT.to_string())
            .basic_auth(&settings.username, Some(&settings.password))
            .query(&query)
            .send()?
            .json()?;

        if category_items
            .get("num")
            .and_then(JsonValue::as_u64)
            .unwrap()
            == 0
        {
            break;
        } else {
            if let Some(book_ids) = category_items.get("book_ids").and_then(JsonValue::as_array) {
                for id in book_ids {
                    if sigterm.load(Ordering::Relaxed) {
                        break;
                    }

                    let url = format!(
                        "{}/ajax/book/{}/{}",
                        &settings.base_url, id, &settings.library
                    );

                    let metadata: JsonValue = client
                        .get(&url)
                        .header(reqwest::header::USER_AGENT, USER_AGENT.to_string())
                        .basic_auth(&settings.username, Some(&settings.password))
                        .query(&query)
                        .send()?
                        .json()?;

                    let title = metadata
                        .get("title")
                        .and_then(JsonValue::as_str)
                        .unwrap_or_default();

                    let author = metadata
                        .get("authors")
                        .and_then(JsonValue::as_array)
                        .unwrap()
                        .iter()
                        .map(|v| v.as_str().unwrap())
                        .collect::<Vec<&str>>()
                        .join(", ");

                    let timestamp = metadata
                        .get("timestamp")
                        .and_then(JsonValue::as_str)
                        .unwrap_or_default()
                        .parse::<DateTime<Utc>>()
                        .unwrap_or(Utc::now());

                    let url_id = metadata
                        .pointer("/identifiers/url")
                        .and_then(JsonValue::as_str)
                        .unwrap_or_default();
                    let hash_id = fxhash::hash64(url_id).to_string();

                    let event = json!({
                        "type": "search",
                        "path": save_path,
                        "query": format!("'i ^{}$", hash_id),
                    });
                    println!("{}", event);
                    let mut line = String::new();
                    io::stdin().read_line(&mut line)?;

                    if let Ok(event) = serde_json::from_str::<JsonValue>(&line) {
                        if let Some(results) = event.get("results").and_then(JsonValue::as_array) {
                            let info = results.first();
                            if let Some(Some(existing_timestamp)) = info.map(|v| v.get("added").and_then(JsonValue::as_str)) {
                                if let Ok(tt) = Utc.datetime_from_str(existing_timestamp, "%Y-%m-%d %H:%M:%S") {
                                    if (tt - timestamp).num_seconds() == 0 {
                                        // Man this is ugly
                                        // If the added time close enough, don't sync
                                        continue;
                                    }
                                }
                            }
                        }
                    }

                    let epub_path = save_path.join(&format!("{}.epub", hash_id));
                    let exists = epub_path.exists();
                    let mut file = File::create(&epub_path)?;
                    let url = format!(
                        "{}/get/EPUB/{}/{}",
                        &settings.base_url, id, &settings.library
                    );

                    let response = client
                        .get(&url)
                        .header(reqwest::header::USER_AGENT, USER_AGENT.to_string())
                        .basic_auth(&settings.username, Some(&settings.password))
                        .send()
                        .and_then(|mut body| body.copy_to(&mut file));

                    if let Err(err) = response {
                        eprintln!("Can't download {}: {:#}.", id, err);
                        fs::remove_file(epub_path).ok();
                        continue;
                    }

                    if let Ok(path) = epub_path.strip_prefix(&library_path) {
                        let file_info = json!({
                            "path": path,
                            "kind": "epub",
                            "size": file.metadata().ok()
                                        .map_or(0, |m| m.len()),
                        });

                        let info = json!({
                            "title": title,
                            "author": author,
                            "identifier": hash_id,
                            "file": file_info,
                            "added": timestamp.with_timezone(&Local)
                                               .format("%Y-%m-%d %H:%M:%S")
                                               .to_string(),
                        });

                        let event = if !exists {
                            json!({
                                "type": "addDocument",
                                "info": &info,
                            })
                        } else {
                            json!({
                                "type": "updateDocument",
                                "path": path,
                                "info": &info,
                            })
                        };

                        println!("{}", event);
                    }
                }
            }
            offset += num;
            query["offset"] = JsonValue::from(offset);
        }
    }
    let message = "Finished syncing books!";
    let event = json!({
        "type": "notify",
        "message": &message,
    });
    println!("{}", event);

    Ok(())
}
