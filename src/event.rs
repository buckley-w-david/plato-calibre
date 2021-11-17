use serde_json::{json, Value as JsonValue};
use std::io;
use std::path::{Path, PathBuf};

use crate::types::Info;

// Events that we send
pub enum Event<'a> {
    Notify(&'a str),
    SetWifi(bool),
    Search {
        path: &'a PathBuf,
        query: String,
    },
    AddDocument(Info),
    UpdateDocument(&'a Path, Info),
}

// Events that we receive
pub enum Response {
    Search(JsonValue),
    NetworkStatus(JsonValue),
}

impl Event<'_> {
    pub fn send(&self) -> Option<Response> {
        let event = match self {
            Event::Notify(msg) => {
                json!({
                    "type": "notify",
                    "message": msg,
                })
            }
            Event::SetWifi(state) => {
                json!({
                    "type": "setWifi",
                    "enable": state,
                })
            }
            Event::Search { path, query } => {
                json!({
                    "type": "search",
                    "path": path,
                    "query": query,
                })
            }
            Event::AddDocument(info) => {
                json!({
                    "type": "addDocument",
                    "info": &info,
                })
            }
            Event::UpdateDocument (path, info) => {
                json!({
                    "type": "updateDocument",
                    "path": path,
                    "info": info,
                })
            }
        };

        println!("{}", event);

        match self {
            Event::Search { .. } => Response::receive(),
            Event::SetWifi { .. } => Response::receive(),
            _ => None,
        }
    }
}

impl Response {
    pub fn receive() -> Option<Response> {
        let mut line = String::new();
        let mut res = None;
        // Yuck
        if let Ok(_) = io::stdin().read_line(&mut line) {
            if let Ok(event) = serde_json::from_str::<JsonValue>(&line) {
                match event.get("type").and_then(JsonValue::as_str) {
                    Some("search") => {
                        res = Some(Response::Search(event));
                    }
                    Some("network") => {
                        res = Some(Response::NetworkStatus(event));
                    }
                    _ => res = None,
                };
            }
        }
        res
    }
}
