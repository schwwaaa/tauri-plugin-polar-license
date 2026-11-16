//! A fake Polar (plus the reclaim Worker) for tests, behaving the way real
//! Polar behaved in production testing:
//!
//! - each key has an activation limit; activations count until released;
//! - **revoke is permanent**: the key and its activations stop validating,
//!   "enable" afterwards changes nothing, the activations keep their slots;
//! - **disable is reversible**: enable makes the key and its activations
//!   valid again;
//! - **rotate** gives the key a new string; status and activations stay;
//! - only genuine Polar 404s use `{"error":"ResourceNotFound"}`; unknown
//!   paths answer a plain HTML 404 (like a proxy or captive portal would).

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use tauri_plugin_polar_license::{Config, Environment, Licensing};

pub const ORG: &str = "00000000-0000-4000-8000-000000000000";
pub const BENEFIT: &str = "11111111-1111-4111-8111-111111111111";
pub const OTHER_BENEFIT: &str = "99999999-9999-4999-8999-999999999999";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Granted,
    Disabled,
    Revoked,
}

#[derive(Debug)]
struct Record {
    benefit: String,
    status: Status,
    limit: usize,
}

#[derive(Debug)]
struct Activation {
    record: usize,
    machine: String,
    dead: bool, // killed by a revoke: counts toward the limit, never valid
}

#[derive(Default)]
struct State {
    records: Vec<Record>,
    keys: HashMap<String, usize>, // key string -> record
    activations: HashMap<String, Activation>,
    next_id: u64,
}

pub struct FakePolar {
    pub base: String,
    state: Arc<Mutex<State>>,
}

fn json(code: u16, body: serde_json::Value) -> (u16, String) {
    (code, body.to_string())
}
fn not_found() -> (u16, String) {
    json(
        404,
        serde_json::json!({"error": "ResourceNotFound", "detail": "Not found"}),
    )
}

impl FakePolar {
    pub fn start() -> Self {
        let server = tiny_http::Server::http("127.0.0.1:0").expect("bind");
        let base = format!("http://{}", server.server_addr().to_ip().unwrap());
        let state = Arc::new(Mutex::new(State::default()));
        let shared = state.clone();
        thread::spawn(move || {
            for mut request in server.incoming_requests() {
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                let (code, text) = handle(&shared, request.url(), &body);
                let content_type = if text.starts_with('<') {
                    "text/html"
                } else {
                    "application/json"
                };
                let response = tiny_http::Response::from_string(text)
                    .with_status_code(code)
                    .with_header(
                        tiny_http::Header::from_bytes("Content-Type", content_type).unwrap(),
                    );
                let _ = request.respond(response);
            }
        });
        Self { base, state }
    }

    /// Issue a key for `benefit` with an activation limit (a "purchase").
    pub fn issue(&self, key: &str, benefit: &str, limit: usize) {
        let mut s = self.state.lock().unwrap();
        s.records.push(Record {
            benefit: benefit.into(),
            status: Status::Granted,
            limit,
        });
        let id = s.records.len() - 1;
        s.keys.insert(key.into(), id);
    }

    pub fn set_status(&self, key: &str, status: Status) {
        let mut s = self.state.lock().unwrap();
        let id = s.keys[key];
        let current = s.records[id].status;
        // Revoke is permanent: nothing changes a revoked key back.
        if current == Status::Revoked {
            return;
        }
        s.records[id].status = status;
        if status == Status::Revoked {
            for a in s.activations.values_mut().filter(|a| a.record == id) {
                a.dead = true;
            }
        }
    }

    /// New key string for the same license; status and activations stay.
    pub fn rotate(&self, old: &str, new: &str) {
        let mut s = self.state.lock().unwrap();
        let id = s.keys.remove(old).unwrap();
        s.keys.insert(new.into(), id);
    }

    pub fn activation_count(&self, key: &str) -> usize {
        let s = self.state.lock().unwrap();
        let id = s.keys[key];
        s.activations.values().filter(|a| a.record == id).count()
    }

    /// A licensing engine for "a computer" (`machine`) storing its file in
    /// `dir`, talking to this fake, with reclaim on or off.
    pub fn computer(&self, dir: &std::path::Path, machine: &str, reclaim: bool) -> Licensing {
        let mut config = Config::new(Environment::custom("sandbox", &self.base, ORG, BENEFIT));
        config.reclaim_url = reclaim.then(|| self.base.clone());
        config.support_contact = Some("support@example.com".into());
        config.log_to_stderr = false;
        Licensing::new(config, dir).with_machine_fingerprint(machine)
    }
}

fn handle(state: &Arc<Mutex<State>>, url: &str, body: &str) -> (u16, String) {
    let b: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    let key = b["key"].as_str().unwrap_or_default().to_string();
    let mut s = state.lock().unwrap();
    if b.get("organization_id").is_some() && b["organization_id"] != ORG {
        return not_found();
    }
    let record = s.keys.get(&key).copied();
    match url {
        "/v1/customer-portal/license-keys/activate" => {
            let Some(id) = record else { return not_found() };
            let r = &s.records[id];
            let used = s.activations.values().filter(|a| a.record == id).count();
            if used >= r.limit {
                return json(
                    403,
                    serde_json::json!({"error": "NotPermitted", "detail": "License key activation limit already reached"}),
                );
            }
            if r.status != Status::Granted {
                return json(
                    403,
                    serde_json::json!({"error": "NotPermitted", "detail": "License key is not active"}),
                );
            }
            s.next_id += 1;
            let act_id = format!("act-{}", s.next_id);
            let machine = b["meta"]["machine"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let benefit = s.records[id].benefit.clone();
            s.activations.insert(
                act_id.clone(),
                Activation {
                    record: id,
                    machine,
                    dead: false,
                },
            );
            json(
                200,
                serde_json::json!({
                    "id": act_id,
                    "label": b["label"],
                    "created_at": "2026-11-15T12:00:00Z",
                    "license_key": { "benefit_id": benefit, "display_key": format!("****-{}", &key[key.len().saturating_sub(6)..]) }
                }),
            )
        }
        "/v1/customer-portal/license-keys/validate" => {
            let Some(id) = record else { return not_found() };
            if s.records[id].status != Status::Granted {
                return not_found();
            }
            match b["activation_id"].as_str() {
                None => json(
                    200,
                    serde_json::json!({"id": format!("lk-{id}"), "status": "granted", "benefit_id": s.records[id].benefit}),
                ),
                Some(act) => match s.activations.get(act) {
                    Some(a) if a.record == id && !a.dead => {
                        json(200, serde_json::json!({"status": "granted"}))
                    }
                    _ => not_found(),
                },
            }
        }
        "/v1/customer-portal/license-keys/deactivate" => {
            let Some(id) = record else { return not_found() };
            let act = b["activation_id"].as_str().unwrap_or_default();
            match s.activations.get(act) {
                Some(a) if a.record == id && s.records[id].status != Status::Revoked => {
                    s.activations.remove(act);
                    (204, String::new())
                }
                _ => not_found(),
            }
        }
        // The reclaim Worker's logic (see worker/src/worker.js).
        "/api/reclaim" => {
            let machine = b["machine"].as_str().unwrap_or_default();
            let Some(id) = record else {
                return json(200, serde_json::json!({"state": "inactive"}));
            };
            if s.records[id].status != Status::Granted {
                return json(200, serde_json::json!({"state": "inactive"}));
            }
            if s.records[id].benefit != BENEFIT {
                return json(200, serde_json::json!({"state": "none"}));
            }
            let mine: Vec<String> = s
                .activations
                .iter()
                .filter(|(_, a)| a.record == id && a.machine == machine)
                .map(|(k, _)| k.clone())
                .collect();
            for a in &mine {
                s.activations.remove(a);
            }
            if mine.is_empty() {
                json(200, serde_json::json!({"state": "none"}))
            } else {
                json(
                    200,
                    serde_json::json!({"state": "reclaimed", "released": mine.len()}),
                )
            }
        }
        _ => (
            404,
            "<html><body>Not found (not Polar)</body></html>".into(),
        ),
    }
}

/// A fresh, empty folder for one "computer's" license file.
pub fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("polar-license-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn run<F: std::future::Future>(f: F) -> F::Output {
    tauri::async_runtime::block_on(f)
}
