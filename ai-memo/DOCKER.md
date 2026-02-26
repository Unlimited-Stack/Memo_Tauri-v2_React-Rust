# 使用 Docker 进行开发与后端编译检查

本目录已内置 Docker 开发环境，目的是避免每位成员在本机/云主机上反复安装 GTK/WebKit 等系统依赖。通过容器可直接：
- 启动前端 Vite 开发服务器（用于手机/浏览器预览 UI），
- 在容器内进行 Rust 后端的 `cargo check`/`cargo build` 验证。

注意：容器内无法直接弹出桌面 GUI（tauri 窗口），因此无法看到“桌面应用窗口”。要体验完整的“🟢 已连接 Rust 底层（支持 IPC）”桌面模式，请在本机原生环境执行 `npm run tauri dev`。

## 先决条件
- 本机安装了 Docker 与 Docker Compose（或 Docker Desktop）。

## 相关文件
- `Dockerfile`：基础镜像基于 Debian Bullseye + Rust，安装了 Node.js 20 与 Tauri v2 在 Linux 下构建所需依赖（libwebkit2gtk-4.0-dev 等）。
- `docker-compose.yml`：定义了两个服务：
  - `dev`：启动前端 Vite 服务（暴露 1420/1421 端口），挂载当前目录代码，适合 Web 预览；
  - `rust-check`：在容器内运行 `cargo check` 进行后端编译检查。
  - `http-server`: 在容器内运行一个独立的 HTTP API 服务（暴露 1422），用于在浏览器/容器模式下持久化与同步备忘录。
- `.dockerignore`：减少构建上下文体积。

## 常用命令
在 `ai-memo/` 目录下执行：

```bash
# 构建镜像（可省略，docker:dev 会自动构建）
npm run docker:build

# 启动前端开发（Vite）并对外暴露 1420/1421 端口
npm run docker:dev
# 打开 http://localhost:1420 进行预览（浏览器中会显示 🟡 纯 Web 预览模式）

# 在容器内对 Rust 后端做一次编译检查
npm run docker:check

# 停止并清理容器
npm run docker:stop
```

## 小贴士
- 若你使用 GitHub Codespaces：默认情况下 Codespaces 容器内不一定允许再运行 Docker（嵌套容器）。这时建议：
  - 使用仓库根目录的 `.devcontainer` 方案来统一依赖，或
  - 在你本地机器运行上述 Docker 命令。
- 在本地桌面环境下，如需完整 Tauri 桌面开发（IPC + 持久化 + 窗口）：
  ```bash
  npm install
  npm run tauri dev
  ```
- 本 Docker 环境已包含常见的 Linux 系统依赖：
  `build-essential pkg-config libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.0-dev libayatana-appindicator3-dev librsvg2-dev patchelf`
