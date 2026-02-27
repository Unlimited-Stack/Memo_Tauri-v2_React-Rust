# 多端部署简易备忘录Test（Tauri v2 架构，Rust + React）

本项目是一个基于 Tauri v2 的极简备忘录应用，支持桌面（Windows/macOS/Linux）、移动（Android/iOS，需本地环境）与纯 Web 预览。前端采用 React + Vite，后端使用 Rust，并通过 Tauri 的 IPC 进行前后端通信。为适配云端开发（如 GitHub Codespaces），前端内置“优雅降级”，即使在没有 Tauri 运行时的浏览器中也能正常预览和交互。

- 前端路径：`ai-memo/src/App.tsx`
- 后端路径：`ai-memo/src-tauri/src/lib.rs`
- 配置与脚手架：`ai-memo/package.json`、`ai-memo/vite.config.ts`、`ai-memo/src-tauri/Cargo.toml`

---

项目特性
- 跨端 UI：一个输入框 + 备忘录列表，最小化依赖、即开即用。
- 优雅降级：浏览器中无 Tauri 运行时时不报错，自动使用纯前端状态。
- IPC 桥接：Tauri v2 `#[tauri::command]` + `@tauri-apps/api` 的 `invoke()` 调用。
- 云端友好：Vite 以固定端口 1420 启动，并支持对外访问（`vite --host`），便于手机真机预览。

---

目录结构（关键文件）
- `src/App.tsx`：前端入口组件，内含 `invoke('add_memo', ...)` 调用与 try/catch 降级逻辑。
- `src-tauri/src/lib.rs`：后端内存状态（Mutex 保护）与 `add_memo` 命令注册。
- `vite.config.ts`：Vite 服务端口固定为 1420，`strictPort: true`。
- `package.json`：`dev` 脚本为 `vite --host`，便于在 Codespaces/云主机提供公网预览。

---

运行方式（一）纯 Web 预览（Codespaces/云主机 推荐）
- 适合快速验证 UI 与交互逻辑（不依赖 Tauri 运行时）。

步骤：
1) 安装依赖并启动 Vite
```bash
npm install
npm run dev
```
2) 在 Codespaces 底部面板打开 Ports（端口）
- 找到 1420 端口，右键“Port Visibility” -> 设为“Public”
- 点击生成的公网链接（通常以 `.github.dev` 结尾）
3) 用手机浏览器访问该链接
- 页面底部会显示：`🟡 纯 Web 预览模式`
- 你可以正常添加备忘录（以纯前端内存状态保存）

说明：
- `vite.config.ts` 已将端口固定为 1420，且脚本 `vite --host` 会监听 0.0.0.0，便于外部访问。

---

运行方式（二）桌面端（前后端打通，IPC 生效）
- 需要在本机安装 Tauri 开发环境与系统依赖。启动后，页面底部会显示 `🟢 已连接 Rust 底层`，前端 `add_memo` 将写入后端内存。

前置（仅 Linux 桌面需安装）：
```bash
sudo apt-get update
sudo apt-get install -y \
	build-essential \
	pkg-config \
	libgtk-3-dev \
	libwebkit2gtk-4.1-dev \
	libayatana-appindicator3-dev \
	librsvg2-dev \
	patchelf
# 若无 4.1 版本，请改用：libwebkit2gtk-4.0-dev
```

运行：
```bash
npm install
npm run tauri dev
```

---

运行方式（三）移动端（Android/iOS，需本机环境）
- Android（需 Android SDK/NDK、Java、Rust Android targets）
```bash
npm install
npm run tauri android dev
```
- iOS（需 macOS + Xcode、Rust iOS targets）
```bash
npm install
npm run tauri ios dev
```

注：以上命令需在本机原生开发环境执行，Codespaces 不适合直接运行移动端模拟器或打包。

---

核心实现摘要
- 前端 `src/App.tsx` 片段（简述）：
	- 维护 `input` 与 `memos` 两个 state
	- `handleAddMemo`：
		- 先尝试 `await invoke('add_memo', { newMemo: input })`（Tauri IPC）
		- 捕获异常时，提示“纯 Web 预览模式”，并以 `setMemos([...prev, input])` 走前端兜底
		- 最后清空输入框
	- 通过 `window.__TAURI_INTERNALS__` 判断显示运行状态提示（Web vs Tauri）

- 后端 `src-tauri/src/lib.rs` 片段（简述）：
	- `MemoState { memos: Mutex<Vec<String>> }`
	- `#[tauri::command] fn add_memo(new_memo: String, state: State<'_, MemoState>) -> Vec<String>`
		- 加锁、push、clone 返回
	- `run()` 中 `.manage(MemoState{...})` 注入状态、`.invoke_handler(generate_handler![add_memo])` 注册命令

---

快速验证清单
- 纯 Web 预览（Codespaces）
	- [ ] `npm install` 后 `npm run dev` 正常输出 Vite 1420 端口
	- [ ] Ports 面板将 1420 设置为 Public
	- [ ] 手机能够访问公网链接并添加备忘录，底部显示 `🟡 纯 Web 预览模式`
- Tauri 桌面（本机）
	- [ ] 安装系统依赖（Linux）并确保 Rust/Tauri 环境就绪
	- [ ] `npm run tauri dev` 能启动桌面应用
	- [ ] 添加备忘录，底部显示 `🟢 已连接 Rust 底层`

---

常见问题与排错
- Linux 构建缺少 `glib-2.0`/GTK/WebKit 相关
	- 安装上文列出的依赖；若 `libwebkit2gtk-4.1-dev` 不存在，改用 `libwebkit2gtk-4.0-dev`
- `invoke` 参数名映射
	- 若遇到参数绑定问题，可将前端 `{ newMemo: input }` 改为 `{ new_memo: input }` 与 Rust 形参精准匹配
- TypeScript 对 `window.__TAURI_INTERNALS__` 提示未声明
	- 可在全局类型声明中扩展 Window 或进行可选链判断

---

版本信息（摘自 `package.json`）
- React ^19.1
- Vite ^7.0
- TypeScript ~5.8
- @tauri-apps/api ^2
- @tauri-apps/cli ^2
- tauri 2.x（Rust 侧）

---

许可
- 本项目仅为示例用途，具体 License 以仓库根目录或后续补充为准。
