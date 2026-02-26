use std::sync::Mutex;
use tauri::State;

// 定义被互斥锁保护的内存状态
struct MemoState {
    memos: Mutex<Vec<String>>,
}

// 暴露给前端的 IPC 接口
#[tauri::command]
fn add_memo(new_memo: String, state: State<'_, MemoState>) -> Vec<String> {
    let mut memos = state.memos.lock().unwrap();
    memos.push(new_memo);
    memos.clone()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // 注入全局内存状态
        .manage(MemoState { memos: Mutex::new(vec![]) })
        // 注册 IPC 指令
        .invoke_handler(tauri::generate_handler![add_memo])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
