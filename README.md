# oh-my-openagent-switch

> 跨平台桌面工具，一键切换 [oh-my-openagent](https://github.com/code-yeongyu/oh-my-openagent) + [opencode](https://opencode.ai) 的供应商配置。

使用大模型编程时，常常需要切换 API 供应商；而 oh-my-openagent 里的 `agents` / `categories` 角色模型也要随之修改。手动同步两个 JSON 容易出错，本工具把整套流程封装成图形化操作。

## 核心功能

- **配置管理** — 增删改查、复制、深拷贝 payload
- **一键应用** — 同时修改 `opencode.jsonc`（深度合并 `provider.omos`）+ 整体替换 `oh-my-openagent.json`
- **自动备份 / 恢复** — 每次应用前自动备份，写入失败保留原文件
- **导入 / 导出** — 从当前 opencode 自动导入；从外部 JSON 文件导入 / 导出到任意路径
- **激活状态检测** — 实时识别当前激活配置、配置漂移、孤立配置
- **跨平台** — macOS / Windows / Linux 桌面应用

## 架构

- **前端** — React 18 + TypeScript + Vite + Tailwind v4 + daisyUI + Zustand + React Router
- **后端** — Tauri 2 (Rust) + jsonc-parser + serde_json + sha2
- **存储** — `~/.config/oh-my-openagent-switch/` 下的 JSON 文件
  - `configs/<id>.json` — 配置正文
  - `backups/<file>-<timestamp>.jsonc` — 备份文件
  - `active.json` — 当前激活配置 + 指纹

## 界面

| 页面 | 职责 |
| --- | --- |
| ListPage | 配置列表 + 激活徽章 + 导入入口 |
| EditPage | 单个配置的表单（provider / agents / categories） |
| BackupsPage | 备份列表 + 恢复 / 删除 |

## 开发

环境要求：Bun ≥ 1.1，Rust stable，Node 仅作备选。

```bash
# 安装依赖
bun install

# 启动开发模式（HMR + Tauri DevTools）
bun run tauri:dev

# 类型检查
bun run typecheck

# Lint
bun run lint

# 格式化
bun run format
```

## 测试

```bash
# Rust 单元 / 集成测试
cd src-tauri && cargo test --all-features

# Rust 静态检查
cd src-tauri && cargo clippy -- -D warnings

# 前端单元测试
bun run test

# Playwright E2E（需要先启动 tauri:dev）
bun run e2e
```

## 构建发布

```bash
# 当前平台
bun run tauri:build

# 三平台打包脚本（产物在 src-tauri/target/release/bundle/）
./scripts/build-mac.sh
./scripts/build-win.sh
./scripts/build-linux.sh
```

推送 `v*` tag 即可触发 GitHub Actions 自动构建并发布 Release 产物（`.dmg` / `.msi` / `.deb` / `.AppImage`）。

## 项目结构

```
.
├── src/                       # React 前端
│   ├── components/            # 通用组件（Layout / Dialog / Toast / ...）
│   ├── pages/                 # 路由页面（List / Edit / Backups）
│   ├── store/                 # Zustand 状态
│   ├── lib/                   # tauri 桥接 + 常量
│   ├── types/                 # 共享类型
│   ├── router.tsx
│   ├── main.tsx
│   └── index.css
├── src-tauri/                 # Rust 后端
│   ├── src/
│   │   ├── commands/          # 12 个 #[tauri::command]
│   │   ├── config/            # JSONC 解析 + 合并 + 指纹
│   │   ├── storage/           # configs / backup / active / paths
│   │   ├── error.rs           # AppError 统一错误类型
│   │   └── lib.rs
│   ├── tests/                 # 集成测试
│   ├── capabilities/          # Tauri 权限声明
│   └── tauri.conf.json
├── scripts/                   # 三平台打包脚本
├── e2e/                       # Playwright 用例
├── .github/workflows/         # CI + Release
├── package.json
├── vite.config.ts
├── playwright.config.ts
└── tsconfig.json
```

## 已知限制（MVP）

- 不含代码签名 / 公证，macOS 首次打开需在「系统设置 › 隐私与安全」手动放行
- 不含自动更新（updater 插件暂未启用）
- 不支持自定义 opencode 路径，仅识别 `~/.config/opencode/`（macOS / Linux）或 `%APPDATA%\opencode\`（Windows）
- 不支持配置分组 / 标签 / 搜索（路线图规划中）

## 路线图

- [ ] 配置分组 + 标签
- [ ] 全局搜索
- [ ] 一键备份清理（保留最近 N 份）
- [ ] 代码签名 + 公证 + 自动更新
- [ ] 多语言（英 / 中）

## 贡献

欢迎 PR。提交前请确保：

1. `bun run lint` / `bun run typecheck` 通过
2. `cd src-tauri && cargo clippy -- -D warnings` 无警告
3. `cd src-tauri && cargo test --all-features` 全绿
4. 涉及 UI 变更请附 Playwright 用例

## 许可

[MIT](./LICENSE) © 2026
