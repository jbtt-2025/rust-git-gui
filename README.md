# Git GUI — 跨平台 Git 图形客户端

基于 **Rust + Tauri 2 + React + TypeScript** 构建的跨平台 Git 图形化客户端，提供直观的提交历史 DAG 可视化、差异对比、分支管理、Blame 追溯等核心功能。

## 截图

> TODO: 添加应用截图

## 功能特性

- **提交历史 DAG 可视化** — Canvas 高性能渲染，支持 100K+ 提交流畅滚动，Solo / Hide / Pin to Left 视图控制
- **差异对比** — 并排 / 内联两种模式，语法高亮（Shiki），Hunk 折叠，图片对比
- **行级暂存** — 文件级和行级暂存 / 取消暂存 / 撤销，精确控制每次提交内容
- **分支管理** — 创建、切换、合并、变基、重命名、删除，拖拽触发 Merge / Rebase
- **远程操作** — Fetch / Pull / Push，自动 stash 流程，冲突解决三方合并视图
- **Blame 视图** — 逐行追溯，颜色编码，时间线视图
- **标签管理** — 轻量标签 / 附注标签，推送到远程
- **Stash 管理** — 创建、应用、弹出、删除，查看 stash 差异
- **Cherry-pick / Revert / Reset** — 支持多提交操作，Soft / Mixed / Hard 三种 Reset 模式
- **操作撤销** — Undo / Redo 引擎，记录 Git 操作历史
- **子模块管理** — 初始化、更新、反初始化，Pull 后自动提示更新
- **Worktree 管理** — 创建、切换、删除
- **内嵌终端** — 基于 xterm.js，自动切换到仓库目录
- **多标签页** — 同时打开多个仓库，拖拽排序
- **主题** — 暗色 / 亮色 / 跟随系统，可配置字体
- **多语言** — 英语、简体中文、日语
- **快捷键** — 全局快捷键，自定义绑定，冲突检测
- **GitHub / GitLab 集成** — Pull Request 列表、创建

## 技术栈

| 层级 | 技术 |
|------|------|
| 后端 | Rust + git2-rs |
| 桥接 | Tauri 2.x |
| 前端 | React 18 + TypeScript |
| 状态管理 | Zustand |
| UI | Radix UI + Tailwind CSS |
| DAG 渲染 | Canvas API |
| 语法高亮 | Shiki |
| 终端 | xterm.js |
| 国际化 | i18next |
| 测试 | proptest (Rust) + fast-check + vitest (前端) |

## 环境要求

- **Node.js** >= 18
- **Rust** >= 1.75（推荐使用 [rustup](https://rustup.rs/) 安装）
- **系统依赖**（按平台）：

### Windows

无额外依赖，确保安装了 [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（含 C++ 桌面开发工作负载）。

### macOS

```bash
xcode-select --install
```

### Linux (Ubuntu/Debian)

```bash
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libssl-dev
```

### Linux (Fedora)

```bash
sudo dnf install webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel openssl-devel
```

### Linux (Arch)

```bash
sudo pacman -S webkit2gtk-4.1 libappindicator-gtk3 librsvg openssl
```

## 快速开始

```bash
# 克隆仓库
git clone https://github.com/your-username/git-gui.git
cd git-gui

# 安装前端依赖
npm install

# 启动开发模式（前端热重载 + Rust 编译）
npm run tauri dev
```

首次启动会编译 Rust 后端，耗时较长（约 2-5 分钟），后续增量编译很快。

## 构建打包

### 本地构建

```bash
# 构建生产版本（自动检测当前平台）
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`：

| 平台 | 产物 |
|------|------|
| Windows | `nsis/git-gui_x.x.x_x64-setup.exe`（安装包）、`msi/git-gui_x.x.x_x64_en-US.msi` |
| macOS | `dmg/git-gui_x.x.x_aarch64.dmg`、`macos/git-gui.app` |
| Linux | `deb/git-gui_x.x.x_amd64.deb`、`appimage/git-gui_x.x.x_amd64.AppImage`、`rpm/git-gui-x.x.x-1.x86_64.rpm` |

### 交叉编译说明

Tauri 不支持交叉编译，需要在目标平台上构建。推荐使用 GitHub Actions 进行多平台 CI/CD（见下方）。

## 测试

```bash
# 前端测试（vitest + fast-check 属性测试）
npm test

# Rust 单元测试 + 属性测试
cd src-tauri
cargo test

# 仅运行 Rust 集成测试
cargo test --test integration_tests
```

## 项目结构

```
git-gui/
├── src/                          # 前端源码 (React + TypeScript)
│   ├── components/               # UI 组件
│   │   ├── CommitGraph/          #   DAG 提交历史图
│   │   ├── DiffViewer/           #   差异对比视图
│   │   ├── BlameViewer/          #   Blame 追溯视图
│   │   ├── Sidebar/              #   侧边栏
│   │   ├── StagingPanel/         #   暂存区面板
│   │   ├── CommitEditor/         #   提交编辑器
│   │   ├── ConflictResolver/     #   冲突解决视图
│   │   ├── TreeBrowser/          #   文件树浏览器
│   │   ├── Terminal/             #   内嵌终端
│   │   ├── TabBar/               #   标签页栏
│   │   ├── Toolbar/              #   工具栏
│   │   ├── Settings/             #   设置面板
│   │   ├── SearchPanel/          #   搜索面板
│   │   ├── CommitContextMenu/    #   右键菜单
│   │   └── DragDrop/             #   拖拽操作
│   ├── stores/                   # Zustand 状态管理
│   ├── ipc/                      # Tauri IPC 通信层
│   ├── hooks/                    # React Hooks
│   ├── i18n/                     # 国际化资源
│   └── themes/                   # 主题样式
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── git_core/             #   git2-rs 封装层
│   │   ├── modules/              #   业务模块
│   │   │   ├── repository_manager.rs
│   │   │   ├── branch_manager.rs
│   │   │   ├── commit_service.rs
│   │   │   ├── diff_service.rs
│   │   │   ├── staging_service.rs
│   │   │   ├── blame_service.rs
│   │   │   ├── remote_manager.rs
│   │   │   ├── stash_manager.rs
│   │   │   ├── tag_manager.rs
│   │   │   ├── rebase_service.rs
│   │   │   ├── undo_engine.rs
│   │   │   └── ...
│   │   ├── ipc.rs                #   Tauri Command 路由
│   │   ├── models.rs             #   数据模型
│   │   └── error.rs              #   错误类型
│   ├── tests/                    #   集成测试
│   └── Cargo.toml
├── package.json
├── vite.config.ts
└── tailwind.config.js
```

## 版本发布

项目使用 GitHub Actions 自动构建和发布。推送 `v*` 标签即可触发：

```bash
# 更新版本号（package.json、Cargo.toml、tauri.conf.json）
# 然后打标签发布
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions 会自动在 Windows、macOS（Intel + Apple Silicon）、Linux 三个平台构建，并将产物上传到 GitHub Releases。

## 许可证

MIT
