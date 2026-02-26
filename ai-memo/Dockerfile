FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

# 安装 Node.js 20（用于前端 Vite / tauri-cli）
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get update \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# 安装 Tauri v2 在 Linux 下构建所需的系统依赖
# 说明：在容器内可以完成 Rust 侧的编译与检查；
# 但容器内无法直接弹出桌面 GUI（tauri 窗口），适合 Web 预览与后端编译验证。
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y \
    build-essential \
    pkg-config \
    curl \
    wget \
    file \
    libssl-dev \
    libglib2.0-dev \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    patchelf \
 && apt-get clean && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 可选：预复制 package.json 以加速首次构建依赖缓存（如果你不想复制，可删除以下两行）
COPY package.json package-lock.json* pnpm-lock.yaml* yarn.lock* ./
RUN if [ -f package.json ]; then npm ci || true; fi

# 默认暴露 Vite 端口（HMR 端口亦可暴露）
EXPOSE 1420 1421

# 容器默认命令：安装依赖并启动 Vite（对外监听）
CMD ["sh", "-lc", "[ -f package.json ] && npm ci; npm run dev -- --host"]
