# Tauri v2 云原生跨平台开发：极简备忘录指南 (Agent 拆解版)

这是专为 **GitHub Codespaces / 云主机** 环境设计的“人机协同”开发流程。通过将复杂架构拆解为四个独立的上下文（Context），我们可以确保 AI Agent 在每一阶段都能精准执行，并实现 **Web 预览挂载 + Rust 后端逻辑** 的完美解耦。

---
## 阶段一：DevContainer 容器化配置与项目初始化
**目标：** 通过环境即代码 (Env as Code) 配置标准的 Tauri 开发容器，彻底解决 Node/Rust 及 Linux 底层依赖的环境差异问题，随后快速构建 Tauri v2 骨架。

### 🤖 第一步：请发给你的 AI Agent（生成云原生环境配置）
> "你现在是我的云原生架构师。我们要在 GitHub Codespaces / 云主机中开发 Tauri v2 应用。我们需要使用 Docker 容器来隔离和标准化开发环境。
> 
> 请帮我在项目根目录创建 `.devcontainer` 文件夹，并生成以下两个文件，为 Tauri 提供包含 Rust、Node.js 20 以及必需 Linux WebKit 依赖的标准化容器环境：
> 
> **1. 创建 `.devcontainer/Dockerfile`，内容如下：**
> ```dockerfile
> FROM [mcr.microsoft.com/devcontainers/rust:1-bullseye](https://mcr.microsoft.com/devcontainers/rust:1-bullseye)
> 
> # 安装 Node.js 20
> RUN curl -fsSL [https://deb.nodesource.com/setup_20.x](https://deb.nodesource.com/setup_20.x) | bash - \
>     && apt-get install -y nodejs
> 
> # 安装 Tauri v2 编译所需的 Linux 系统依赖
> RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
>     && apt-get install -y \
>     libwebkit2gtk-4.0-dev \
>     build-essential \
>     curl \
>     wget \
>     file \
>     libssl-dev \
>     libgtk-3-dev \
>     libayatana-appindicator3-dev \
>     librsvg2-dev \
>     && apt-get clean && rm -rf /var/lib/apt/lists/*
> ```
> 
> **2. 创建 `.devcontainer/devcontainer.json`，内容如下：**
> ```json
> {
>   "name": "Tauri v2 Dev Environment",
>   "build": { "dockerfile": "Dockerfile" },
>   "customizations": {
>     "vscode": {
>       "extensions": [
>         "rust-lang.rust-analyzer",
>         "tauri-apps.tauri-vscode",
>         "dbaeumer.vscode-eslint"
>       ]
>     }
>   },
>   "forwardPorts": [1420],
>   "remoteUser": "vscode"
> }
> ```
> 请在文件创建完成后通知我。"

---

### 🛠️ 开发者手动操作：重建容器 (极速拉取环境)
当 Agent 创建好 `.devcontainer` 目录后：
1. 在网页端 Codespaces 或 VS Code 中按下 `Ctrl+Shift+P` (Mac 为 `Cmd+Shift+P`)。
2. 输入并选择 **"Dev Containers: Rebuild Container"** (重建容器)。
3. 喝口水等待一两分钟，云主机将自动根据 Dockerfile 拉取镜像并配置好完美的 Tauri 环境。（此步骤彻底取代了手动配置环境）。

---

### 🤖 第二步：请发给你的 AI Agent（容器重启后，初始化项目）
> "太棒了，现在我们的云原生容器环境已经就绪，`node`、`rustc` 以及 Linux 底层构建依赖均已就位。
> 
> 请帮我执行以下操作：
> 1. 使用 npm 帮我初始化一个 Tauri v2 项目，项目名称为 `ai-memo`，前端框架选择 **React**，语言选择 **TypeScript**。
>    *命令参考：* `npm create tauri-app@latest ai-memo -- --manager npm --template react-ts`
> 2. 进入 `ai-memo` 目录，运行 `npm install` 安装所有前端依赖。
> 3. 完成后，告诉我目录结构是否生成成功。"

---

## 阶段二：编写跨端 Web UI 与 IPC 桥接 (带降级兼容)
**目标：** 编写 React 前端界面，并加入对浏览器预览友好的“优雅降级”逻辑，防止在非 Tauri 环境下崩溃。

### 🤖 请发给你的 AI Agent：
> "太棒了。现在请帮我改写前端 UI 代码。
> 
> 请将 `src/App.tsx` 的内容完全替换为以下代码。这段代码包含了一个输入框和列表，并在调用 Tauri 的 `invoke` 时做了 `try-catch` 降级处理，确保我们在普通手机浏览器里也能预览 UI。
> 
> ```typescript
> import { useState } from 'react';
> import { invoke } from '@tauri-apps/api/core';
> 
> export default function App() {
>   const [input, setInput] = useState('');
>   const [memos, setMemos] = useState<string[]>([]);
> 
>   const handleAddMemo = async () => {
>     if (!input.trim()) return;
>     
>     try {
>       // 尝试跨语言调用 Rust 后端
>       const updatedMemos: string[] = await invoke('add_memo', { newMemo: input });
>       setMemos(updatedMemos);
>     } catch (error) {
>       // 【优雅降级】如果在普通手机浏览器里预览，走纯前端逻辑
>       console.warn("当前处于纯 Web 预览模式，Rust 后端未连接。使用纯前端状态。");
>       setMemos(prev => [...prev, input]);
>     } finally {
>       setInput('');
>     }
>   };
> 
>   return (
>     <div style={{ padding: '20px', fontFamily: 'system-ui', maxWidth: '400px', margin: '0 auto' }}>
>       <h2 style={{ color: '#333' }}>✨ AI Agent 跨端备忘录</h2>
>       <div style={{ display: 'flex', gap: '10px' }}>
>         <input
>           value={input}
>           onChange={(e) => setInput(e.target.value)}
>           placeholder="输入备忘录内容..."
>           style={{ flex: 1, padding: '10px', borderRadius: '8px', border: '1px solid #ccc', fontSize: '16px' }}
>         />
>         <button 
>           onClick={handleAddMemo} 
>           style={{ padding: '10px 16px', borderRadius: '8px', background: '#0070f3', color: '#fff', border: 'none', fontWeight: 'bold' }}
>         >
>           记录
>         </button>
>       </div>
>       
>       <ul style={{ marginTop: '20px', paddingLeft: '0', listStyle: 'none' }}>
>         {memos.map((memo, idx) => (
>           <li key={idx} style={{ padding: '12px', borderBottom: '1px solid #eee', background: '#f9f9f9', marginBottom: '8px', borderRadius: '6px' }}>
>             {memo}
>           </li>
>         ))}
>       </ul>
>       <p style={{ fontSize: '12px', color: '#888', marginTop: '20px' }}>
>         运行环境状态: {window.__TAURI_INTERNALS__ ? '🟢 已连接 Rust 底层' : '🟡 纯 Web 预览模式'}
>       </p>
>     </div>
>   );
> }
> ```
> 
> 请执行替换，并确认替换成功。"

---

## 阶段三：编写后端 Rust 内存驻留逻辑
**目标：** 利用 Rust 的 `Mutex` (互斥锁) 构建内存安全的数据处理核心。

### 🤖 请发给你的 AI Agent：
> "前端搞定了，现在来写后端核心。
> 
> 请找到 `src-tauri/src/lib.rs`，将其内容完全替换为以下 Rust 代码。我们要建立一个由 Mutex 保护的内存备忘录状态，并通过 `tauri::command` 暴露给前端。
> 
> ```rust
> use std::sync::Mutex;
> use tauri::State;
> 
> // 定义被互斥锁保护的内存状态
> struct MemoState {
>     memos: Mutex<Vec<String>>,
> }
> 
> // 暴露给前端的 IPC 接口
> #[tauri::command]
> fn add_memo(new_memo: String, state: State<'_, MemoState>) -> Vec<String> {
>     let mut memos = state.memos.lock().unwrap();
>     memos.push(new_memo);
>     memos.clone()
> }
> 
> #[cfg_attr(mobile, tauri::mobile_entry_point)]
> pub fn run() {
>     tauri::Builder::default()
>         .plugin(tauri_plugin_opener::init())
>         // 注入全局内存状态
>         .manage(MemoState { memos: Mutex::new(vec![]) })
>         // 注册 IPC 指令
>         .invoke_handler(tauri::generate_handler![add_memo])
>         .run(tauri::generate_context!())
>         .expect("error while running tauri application");
> }
> ```
> 
> 请执行替换，运行 `cargo check` (在 `src-tauri` 目录下) 确保 Rust 代码没有语法错误，并向我报告结果。"

---

## 阶段四：启动 Vite 开发服务器与真机预览
**目标：** 在云端启动服务，并暴露给公网，让你用手机浏览器直接访问。

### 🤖 请发给你的 AI Agent：
> "代码都已经就绪。现在我们要验证云端到手机终端的 UI 连通性。
> 
> 1. 请帮我修改前端的 `package.json`，将 `dev` 脚本修改为 `"dev": "vite --host"`。
> 2. 请在终端中运行 `npm run dev` 启动前端服务器。
> 3. 请告诉我服务运行在哪个端口（通常是 1420），并指导我如何通过当前云平台（如 Codespaces 的端口转发）获取公网 URL。"

---

## 📱 开发者手动验证步骤
当 Agent 完成上述操作后：
1. 在 Codespaces 底部面板找到 **"Ports (端口)"** 标签页。
2. 找到 **1420** 端口，右键选择 **"Port Visibility"** -> **"Public"**。
3. 点击生成的公网链接（通常以 `.github.dev` 结尾）。
4. **验证时刻：** 用手机扫描该链接。你会看到底部的状态显示为 `🟡 纯 Web 预览模式`。此时你可以像原生 App 一样测试 UI 响应式和交互逻辑。