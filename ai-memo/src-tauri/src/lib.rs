use std::{fs, io::Write, path::PathBuf, sync::{Arc, Mutex}, time::{SystemTime, UNIX_EPOCH}, thread, io::Read};
use serde::{Deserialize, Serialize};
use tauri::{path::BaseDirectory, AppHandle, State};
use tiny_http::{Server, Response, Method, StatusCode, Header};
use tauri::Manager;
fn add_cors<R: Read>(mut resp: Response<R>) -> Response<R> {
    let h_origin = Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap();
    let h_methods = Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET,POST,PUT,DELETE,OPTIONS"[..]).unwrap();
    let h_headers = Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type"[..]).unwrap();
    resp.add_header(h_origin);
    resp.add_header(h_methods);
    resp.add_header(h_headers);
    resp
}

// 备忘录结构：包含唯一 id、文本内容、完成状态
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Memo {
    id: String,
    text: String,
    completed: bool,
}

// 由互斥锁保护的全局内存状态（使用 Arc 以便在 HTTP 线程中共享）
struct MemoState {
    memos: Arc<Mutex<Vec<Memo>>>,
}

// 生成简单唯一 ID（基于时间戳）
fn gen_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("m{}", ts)
}

fn data_file_path(app: &AppHandle) -> PathBuf {
    // 将数据保存在 AppData 目录下的 memos.json
    app.path()
        .resolve("memos.json", BaseDirectory::AppData)
        .expect("failed to resolve app data path")
}

fn save_to_disk(app: &AppHandle, memos: &Vec<Memo>) {
    let path = data_file_path(app);
    if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
    if let Ok(json) = serde_json::to_string_pretty(memos) {
        if let Ok(mut file) = fs::File::create(&path) {
            let _ = file.write_all(json.as_bytes());
        }
    }
}

fn load_from_disk(app: &AppHandle) -> Vec<Memo> {
    let path = data_file_path(app);
    match fs::read_to_string(path) {
        Ok(s) => serde_json::from_str::<Vec<Memo>>(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

// 读取全部备忘录
#[tauri::command]
fn get_memos(state: State<'_, MemoState>) -> Vec<Memo> {
    state.memos.lock().unwrap().clone()
}

// 新增备忘录
#[tauri::command]
fn add_memo(text: String, state: State<'_, MemoState>, app: AppHandle) -> Vec<Memo> {
    let mut memos = state.memos.lock().unwrap();
    memos.push(Memo { id: gen_id(), text, completed: false });
    let cloned = memos.clone();
    drop(memos);
    save_to_disk(&app, &cloned);
    cloned
}

// 勾选/反选完成
#[tauri::command]
fn toggle_complete(id: String, state: State<'_, MemoState>, app: AppHandle) -> Vec<Memo> {
    let mut memos = state.memos.lock().unwrap();
    if let Some(m) = memos.iter_mut().find(|m| m.id == id) {
        m.completed = !m.completed;
    }
    let cloned = memos.clone();
    drop(memos);
    save_to_disk(&app, &cloned);
    cloned
}

// 编辑文本
#[tauri::command]
fn edit_memo(id: String, text: String, state: State<'_, MemoState>, app: AppHandle) -> Vec<Memo> {
    let mut memos = state.memos.lock().unwrap();
    if let Some(m) = memos.iter_mut().find(|m| m.id == id) {
        m.text = text;
    }
    let cloned = memos.clone();
    drop(memos);
    save_to_disk(&app, &cloned);
    cloned
}

// 删除
#[tauri::command]
fn delete_memo(id: String, state: State<'_, MemoState>, app: AppHandle) -> Vec<Memo> {
    let mut memos = state.memos.lock().unwrap();
    memos.retain(|m| m.id != id);
    let cloned = memos.clone();
    drop(memos);
    save_to_disk(&app, &cloned);
    cloned
}

// 重排（根据前端传入的新顺序 id 列表）
#[tauri::command]
fn reorder_memos(order: Vec<String>, state: State<'_, MemoState>, app: AppHandle) -> Vec<Memo> {
    let mut memos = state.memos.lock().unwrap();
    let mut map = std::collections::HashMap::new();
    for m in memos.iter().cloned() { map.insert(m.id.clone(), m); }
    let mut new_list = Vec::with_capacity(order.len());
    for id in order {
        if let Some(m) = map.remove(&id) { new_list.push(m); }
    }
    // 附加未出现在 order 中的（容错）
    for (_k, v) in map.into_iter() { new_list.push(v); }
    *memos = new_list;
    let cloned = memos.clone();
    drop(memos);
    save_to_disk(&app, &cloned);
    cloned
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // 在 setup 阶段从磁盘加载已有数据，并注入全局状态，同时启动一个内置的 HTTP API（供容器/浏览器模式使用）
        .setup(|app| {
            let handle = app.handle();
            let initial = load_from_disk(&handle);
            let memos_arc = Arc::new(Mutex::new(initial));

            // 克隆以注入到 tauri 管理状态
            app.manage(MemoState { memos: memos_arc.clone() });

            // 启动一个简单的 HTTP 服务器，监听 1422，提供 RESTful 接口
            let handle_for_thread = handle.clone();
            let memos_for_thread = memos_arc.clone();
            thread::spawn(move || {
                if let Ok(server) = Server::http("0.0.0.0:1422") {
                    for mut request in server.incoming_requests() {
                        let method = request.method().clone();
                        let url = request.url().to_string();
                        // 预检请求（CORS）
                        if method == Method::Options {
                            let resp = add_cors(Response::from_string(String::new()).with_status_code(204));
                            let _ = request.respond(resp);
                            continue;
                        }
                        // 使用 from_string 创建可变 response，保持与后续 from_string 类型一致
                        let mut response = Response::from_string(String::new());

                        // 简单路由与处理
                        match (method, url.as_str()) {
                            (Method::Get, "/api/memos") => {
                                let list = memos_for_thread.lock().unwrap().clone();
                                if let Ok(body) = serde_json::to_string(&list) {
                                    response = Response::from_string(body);
                                    response = response.with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                                    response = add_cors(response);
                                }
                            }
                            (Method::Post, "/api/memos") => {
                                // 读取 body
                                let mut content = String::new();
                                let _ = request.as_reader().read_to_string(&mut content);
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                                    if let Some(text) = v.get("text").and_then(|t| t.as_str()) {
                                        let mut memos = memos_for_thread.lock().unwrap();
                                        memos.push(Memo { id: gen_id(), text: text.to_string(), completed: false });
                                        let cloned = memos.clone();
                                        drop(memos);
                                        save_to_disk(&handle_for_thread, &cloned);
                                        if let Ok(body) = serde_json::to_string(&cloned) {
                                            response = Response::from_string(body);
                                            response = response.with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                                            response = add_cors(response);
                                        }
                                    } else {
                                        response = add_cors(Response::from_string(String::new()).with_status_code(400));
                                    }
                                } else { response = add_cors(Response::from_string(String::new()).with_status_code(400)); }
                            }
                            (Method::Post, "/api/memos/toggle") => {
                                let mut content = String::new();
                                let _ = request.as_reader().read_to_string(&mut content);
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                                    if let Some(id) = v.get("id").and_then(|t| t.as_str()) {
                                        let mut memos = memos_for_thread.lock().unwrap();
                                        if let Some(m) = memos.iter_mut().find(|m| m.id == id) { m.completed = !m.completed; }
                                        let cloned = memos.clone();
                                        drop(memos);
                                        save_to_disk(&handle_for_thread, &cloned);
                                        if let Ok(body) = serde_json::to_string(&cloned) {
                                            response = Response::from_string(body);
                                            response = response.with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                                            response = add_cors(response);
                                        }
                                    } else { response = add_cors(Response::from_string(String::new()).with_status_code(400)); }
                                } else { response = add_cors(Response::from_string(String::new()).with_status_code(400)); }
                            }
                            (Method::Put, url) if url.starts_with("/api/memos/") => {
                                let id = url.trim_start_matches("/api/memos/");
                                let mut content = String::new();
                                let _ = request.as_reader().read_to_string(&mut content);
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                                    if let Some(text) = v.get("text").and_then(|t| t.as_str()) {
                                        let mut memos = memos_for_thread.lock().unwrap();
                                        if let Some(m) = memos.iter_mut().find(|m| m.id == id) { m.text = text.to_string(); }
                                        let cloned = memos.clone();
                                        drop(memos);
                                        save_to_disk(&handle_for_thread, &cloned);
                                        if let Ok(body) = serde_json::to_string(&cloned) {
                                            response = Response::from_string(body);
                                            response = response.with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                                            response = add_cors(response);
                                        }
                                    } else { response = add_cors(Response::from_string(String::new()).with_status_code(400)); }
                                } else { response = add_cors(Response::from_string(String::new()).with_status_code(400)); }
                            }
                            (Method::Delete, url) if url.starts_with("/api/memos/") => {
                                let id = url.trim_start_matches("/api/memos/");
                                let mut memos = memos_for_thread.lock().unwrap();
                                memos.retain(|m| m.id != id);
                                let cloned = memos.clone();
                                drop(memos);
                                save_to_disk(&handle_for_thread, &cloned);
                                if let Ok(body) = serde_json::to_string(&cloned) {
                                    response = Response::from_string(body);
                                    response = response.with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                                    response = add_cors(response);
                                }
                            }
                            (Method::Post, "/api/memos/reorder") => {
                                let mut content = String::new();
                                let _ = request.as_reader().read_to_string(&mut content);
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                                    if let Some(arr) = v.as_object().and_then(|o| o.get("order")) {
                                        if let Some(order) = arr.as_array() {
                                            let order_ids: Vec<String> = order.iter().filter_map(|it| it.as_str().map(|s| s.to_string())).collect();
                                            let mut memos = memos_for_thread.lock().unwrap();
                                            let mut map = std::collections::HashMap::new();
                                            for m in memos.iter().cloned() { map.insert(m.id.clone(), m); }
                                            let mut new_list = Vec::new();
                                            for id in order_ids { if let Some(m) = map.remove(&id) { new_list.push(m); } }
                                            for (_k, v) in map.into_iter() { new_list.push(v); }
                                            *memos = new_list;
                                            let cloned = memos.clone();
                                            drop(memos);
                                            save_to_disk(&handle_for_thread, &cloned);
                                            if let Ok(body) = serde_json::to_string(&cloned) {
                                                response = Response::from_string(body);
                                                response = response.with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                                                response = add_cors(response);
                                            }
                                        } else { response = add_cors(Response::from_string(String::new()).with_status_code(400)); }
                                    } else { response = add_cors(Response::from_string(String::new()).with_status_code(400)); }
                                } else { response = add_cors(Response::from_string(String::new()).with_status_code(400)); }
                            }
                            _ => {
                                response = add_cors(Response::from_string(String::new()).with_status_code(404));
                            }
                        }

                        let _ = request.respond(response);
                    }
                }
            });

            Ok(())
        })
        // 注册 IPC 指令
        .invoke_handler(tauri::generate_handler![
            get_memos,
            add_memo,
            toggle_complete,
            edit_memo,
            delete_memo,
            reorder_memos
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
