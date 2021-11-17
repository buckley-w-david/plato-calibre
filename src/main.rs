mod settings;
mod event;
mod calibre;

use anyhow::{format_err, Context, Error};
use chrono::prelude::*;
use chrono::Local;
use reqwest::blocking::Client;
use serde_json::{json, Value as JsonValue};
use std::env;
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use const_format::concatcp;
use event::{Event, Response};

const SETTINGS_PATH: &str = "Settings.toml";

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const NAME: &'static str = env!("CARGO_PKG_NAME");
const USER_AGENT: &'static str = concatcp!(NAME, " ", VERSION);

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

    let settings = settings::load_toml::<settings::Settings, _>(SETTINGS_PATH)
        .with_context(|| format!("can't load settings from {}", SETTINGS_PATH))?;

    if !online {
        if !wifi {
            Event::Notify("Establishing a network connection.").send();
            Event::SetWifi(true).send();
        } else {
            Event::Notify("Waiting for the network to come up.").send();
            // Throw away network coming up event
            let _event = Response::receive();
        }
    }

    if !save_path.exists() {
        fs::create_dir(&save_path)?;
    }

    let client = Client::new();

    let sigterm = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&sigterm))?;

    for id in calibre::books_in(&client, &settings) {
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

        if let Some(Response::Search(event)) = (Event::Search{ path: &save_path, query: format!("'i ^{}$", hash_id)}).send() {
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
                Event::AddDocument(&info)
            } else {
                Event::UpdateDocument{path: &path, info: &info}
            };
            event.send();
        }
        break;
    }

    Event::Notify("Finished syncing books!").send();

    Ok(())
}
