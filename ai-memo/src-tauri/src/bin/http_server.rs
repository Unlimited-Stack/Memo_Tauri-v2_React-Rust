use std::{fs, io::Write, path::PathBuf, sync::{Arc, Mutex}};
use tiny_http::{Server, Response, Method};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Memo { id: String, text: String, completed: bool }

fn data_path() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")) .join("memos.json")
}

fn gen_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    format!("m{}", ts)
}

fn load(path: &PathBuf) -> Vec<Memo> {
    match fs::read_to_string(path) { Ok(s) => serde_json::from_str(&s).unwrap_or_default(), Err(_) => Vec::new() }
}

fn save(path: &PathBuf, memos: &Vec<Memo>) {
    if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
    if let Ok(json) = serde_json::to_string_pretty(memos) {
        if let Ok(mut f) = fs::File::create(path) { let _ = f.write_all(json.as_bytes()); }
    }
}

fn with_cors(mut resp: Response<std::io::Cursor<Vec<u8>>>) -> Response<std::io::Cursor<Vec<u8>>> {
    resp.add_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap());
    resp.add_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET,POST,PUT,DELETE,OPTIONS"[..]).unwrap());
    resp.add_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type"[..]).unwrap());
    resp
}

fn main() {
    let path = data_path();
    let memos = Arc::new(Mutex::new(load(&path)));

    let server = Server::http("0.0.0.0:1422").expect("bind");
    eprintln!("http server listening on 0.0.0.0:1422, data: {}", path.display());

    for mut req in server.incoming_requests() {
        let method = req.method().clone();
        let url = req.url().to_string();
        if method == Method::Options {
            let resp = Response::from_string(String::new()).with_status_code(204);
            let _ = req.respond(with_cors(resp));
            continue;
        }

        // route
        if method == Method::Get && url == "/api/memos" {
            let list = memos.lock().unwrap().clone();
            if let Ok(body) = serde_json::to_string(&list) {
                let resp = Response::from_string(body);
                let _ = req.respond(with_cors(resp));
                continue;
            }
        }

        if method == Method::Post && url == "/api/memos" {
            let mut s = String::new(); let _ = req.as_reader().read_to_string(&mut s);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                if let Some(text) = v.get("text").and_then(|t| t.as_str()) {
                    let mut m = memos.lock().unwrap();
                    m.push(Memo { id: gen_id(), text: text.to_string(), completed: false });
                    let cloned = m.clone(); drop(m);
                    save(&path, &cloned);
                    let resp = Response::from_string(serde_json::to_string(&cloned).unwrap());
                    let _ = req.respond(with_cors(resp));
                    continue;
                }
            }
            let _ = req.respond(with_cors(Response::from_string(String::new()).with_status_code(400)));
            continue;
        }

        if method == Method::Post && url == "/api/memos/toggle" {
            let mut s = String::new(); let _ = req.as_reader().read_to_string(&mut s);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                if let Some(id) = v.get("id").and_then(|t| t.as_str()) {
                    let mut m = memos.lock().unwrap(); if let Some(it) = m.iter_mut().find(|x| x.id==id) { it.completed = !it.completed; }
                    let cloned = m.clone(); drop(m);
                    save(&path, &cloned);
                    let _ = req.respond(with_cors(Response::from_string(serde_json::to_string(&cloned).unwrap())));
                    continue;
                }
            }
            let _ = req.respond(with_cors(Response::from_string(String::new()).with_status_code(400)));
            continue;
        }

        if method == Method::Put && url.starts_with("/api/memos/") {
            let id = url.trim_start_matches("/api/memos/");
            let mut s = String::new(); let _ = req.as_reader().read_to_string(&mut s);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                if let Some(text) = v.get("text").and_then(|t| t.as_str()) {
                    let mut m = memos.lock().unwrap(); if let Some(it) = m.iter_mut().find(|x| x.id==id) { it.text = text.to_string(); }
                    let cloned = m.clone(); drop(m); save(&path, &cloned);
                    let _ = req.respond(with_cors(Response::from_string(serde_json::to_string(&cloned).unwrap())));
                    continue;
                }
            }
            let _ = req.respond(with_cors(Response::from_string(String::new()).with_status_code(400)));
            continue;
        }

        if method == Method::Delete && url.starts_with("/api/memos/") {
            let id = url.trim_start_matches("/api/memos/");
            let mut m = memos.lock().unwrap(); m.retain(|x| x.id!=id); let cloned = m.clone(); drop(m); save(&path, &cloned);
            let _ = req.respond(with_cors(Response::from_string(serde_json::to_string(&cloned).unwrap())));
            continue;
        }

        if method == Method::Post && url == "/api/memos/reorder" {
            let mut s = String::new(); let _ = req.as_reader().read_to_string(&mut s);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                if let Some(arr) = v.get("order").and_then(|o| o.as_array()) {
                    let order_ids: Vec<String> = arr.iter().filter_map(|it| it.as_str().map(|s| s.to_string())).collect();
                    let mut m = memos.lock().unwrap(); let mut map = std::collections::HashMap::new(); for itm in m.iter().cloned() { map.insert(itm.id.clone(), itm); } let mut new_list = Vec::new(); for id in order_ids { if let Some(itm) = map.remove(&id) { new_list.push(itm); } } for (_k,v) in map.into_iter() { new_list.push(v); } *m = new_list; let cloned = m.clone(); drop(m); save(&path, &cloned);
                    let _ = req.respond(with_cors(Response::from_string(serde_json::to_string(&cloned).unwrap())));
                    continue;
                }
            }
            let _ = req.respond(with_cors(Response::from_string(String::new()).with_status_code(400)));
            continue;
        }

        let _ = req.respond(with_cors(Response::from_string(String::new()).with_status_code(404)));
    }
}
