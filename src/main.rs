mod calibre;
mod error;
mod event;
mod settings;
mod types;

use anyhow::{format_err, Context, Error};
use chrono::prelude::*;
use reqwest::blocking::Client;
use std::env;
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use calibre::ContentServer;
use event::{Event, Response};
use types::{FileInfo, Info};
use error::PlatoCalibreError;

const SETTINGS_PATH: &str = "Settings.toml";

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

    let settings::Settings {
        base_url,
        username,
        password,
        ..
    } = settings;
    let content_server = ContentServer::new(Client::new(), base_url, username, password);

    let sigterm = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&sigterm))?;

    for id in content_server.books_in(settings.category, settings.item, &settings.library) {
        if sigterm.load(Ordering::Relaxed) {
            break;
        }

        let metadata = content_server.metadata(id, &settings.library)?;

        let calibre_id = if let Some(identifier) = &settings.identifier {
            &metadata
                .identifiers
                .get(identifier)
                .ok_or(PlatoCalibreError::new("Unable to find identifier"))?
        } else {
            &metadata.title
        };
        let hash_id = fxhash::hash64(calibre_id).to_string();

        if let Some(Response::Search(event)) = (Event::Search {
            path: &save_path,
            query: format!("'i ^{}$", hash_id),
        })
        .send()
        {
            if let Some(info) = event.results.first() {
                if info.added == metadata.timestamp.with_nanosecond(0).unwrap() {
                    println!("Skipping!");
                    continue;
                }
            }
        }

        let epub_path = save_path.join(&format!("{}.epub", hash_id));
        let exists = epub_path.exists();
        let mut file = File::create(&epub_path)?;

        let response = content_server.epub(id, &settings.library, &mut file);

        if let Err(err) = response {
            eprintln!("Can't download {}: {:#}.", id, err);
            fs::remove_file(epub_path).ok();
            continue;
        }

        if let Ok(path) = epub_path.strip_prefix(&library_path) {
            let file_info = FileInfo {
                path: path.to_path_buf(),
                kind: "epub".to_string(),
                size: file.metadata().ok().map_or(0, |m| m.len()),
            };

            let info = Info {
                title: metadata.title,
                author: metadata.author,
                identifier: hash_id,
                file: file_info,
                added: metadata.timestamp.into(),
            };

            let event = if !exists {
                Event::AddDocument(info)
            } else {
                Event::UpdateDocument(path, info)
            };
            event.send();
        }
    }

    Event::Notify("Finished syncing books!").send();

    Ok(())
}
