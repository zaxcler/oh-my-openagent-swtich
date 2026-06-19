# oh-my-openagent-switch

## TL;DR

> **Quick Summary**: 基于 Tauri v2 的跨平台桌面 GUI（macOS/Windows/Linux），用于管理多个"供应商配置"，一键切换时同步修改 `~/.config/opencode/oh-my-openagent.json`（整体替换）和 `~/.config/opencode/opencode.jsonc` 的 `provider.omos` 块（深度合并）。
>
> **Deliverables**:
> - 桌面应用：列表页 + 编辑页 + 应用/导入/导出/备份
> - 5+ 个 Tauri 命令（list/get/create/update/apply/import/export/backup）
> - 12 个合并边界场景 100% 单元测试覆盖
> - 关键 E2E 场景用 Playwright + tauri-driver
> - 三平台打包脚本
>
> **Estimated Effort**: Medium-Large
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: T1 → T5 → T6 → T9 → T10 → T12 → T13 → T17 → F1-F4

---

## Context

### Original Request
用户在做 opencode + oh-my-openagent 编程时，需要频繁切换大模型供应商（不同供应商的 apiKey、baseURL、可用模型不同），同时 oh-my-openagent 内部的"角色模型"配置（每个 agent 角色用哪个 model）也要跟着切换。手动改两个 JSON 文件非常痛苦，所以要一个桌面 GUI 工具。

### Interview Summary
**Key Decisions**:
- 框架：Tauri v2（体积 3-15MB，满足"最小型"硬性要求）
- 平台：macOS + Windows + Linux
- 持久化：一个配置 = 一个 JSON 文件，存于 `~/Library/Application Support/oh-my-openagent-switch/configs/`（mac）
- 命名：显示名（友好型）+ UUID 文件名（避免冲突）
- "保存"语义：**保存 + 单独应用按钮**（保存写工具自己的库，应用才写 opencode 目录）
- 备份：应用前自动备份到 `.../oh-my-openagent-switch/backups/`
- 激活提示：列表显示「✓ 激活」，启动时自动检测
- 导入/导出：支持，方便分享

**Research Findings**:
- Tauri v2 体积 3-15MB、启动 0.2-0.5s、社区 107K ⭐、官方支持 tauri-driver
- 配置文件位置（macOS/Linux）`~/.config/opencode/`、（Windows）`%APPDATA%\opencode\`
- OpenCode **不热重载**，修改配置后必须重启
- 项目说明中 `omos` 是**固定 provider key**（不是我之前以为的用户填的 key）

### Metis Review - Critical Corrections

**1. provider key 错误理解修正**：
- 我之前误以为"用户填 provider key"
- **实际**：项目说明明确"omos 固定值，生成 json 时他是 key，不需要用户填写"
- 用户界面**不暴露** provider key 字段

**2. models 结构修正**：
- 我之前说"models 数组"
- **实际**：models 是**对象 map**（`{ "model-id": { "name": "..." } }`）
- 但 UI 上仍以"列表行"形式让用户增删改（每行一个 model）

**3. 关键隐藏风险**（影响合并策略）：
- 用户的 `opencode.jsonc` 中 model 可能含 `headers` / `limit` / `whitelist` 等 opencode 已知但项目说明未列的字段
- **不能整体覆盖** provider.omos 块，必须做**深度合并**（保留 target.model 的非 name 字段）
- 12 个必测边界场景已识别

**4. 关键设计决策**（已采纳 Metis 建议）：
- 激活状态：**B 为主**（active.json 记录） + **A 兜底**（指纹对比检测外部修改）
- 写入策略：**tempfile + rename 原子操作**
- 备份：**强制**（失败拒绝应用）
- JSON 操作：**用 `serde_json::Value` 而非强类型反序列化**（保留所有未知字段）
- JSONC：用 `jsonc-parser` 保留注释
- 错误处理：统一 `AppError` + `Serialize` 到前端

### Metis Open Decisions（已在计划中给出默认）
| 决策点 | 默认 | 用户可改 |
|---|---|---|
| 源中删除的 model 在目标中 | **保留**（保守） | 编辑页加 checkbox "清理未引用 model" |
| 进程检测 | **best-effort 提示**（不阻塞） | 写命令直接做 |
| 自定义 opencode 路径 | **MVP 不做** | v2 加 |
| 代码签名 | **MVP 不做**（自用优先） | v2 加 |

---

## Work Objectives

### Core Objective
为 oh-my-openagent + opencode 用户提供一个跨平台桌面 GUI 工具，简化"切换供应商 + 切换角色模型"流程。

### Concrete Deliverables
- `.omo/plans/oh-my-openagent-switch.md`（本文件）
- `src-tauri/`：Rust 后端
  - `commands/`：Tauri 命令
  - `config/`：合并、指纹、JSONC 工具
  - `storage/`：CRUD、备份、路径解析
- `src/`：React 前端
  - `pages/`：列表页、编辑页
  - `components/`：表单、按钮、对话框
  - `lib/`：tauri invoke 包装、状态管理
- `tests/`：Rust 单元测试
- `e2e/`：Playwright E2E
- `package.json` / `Cargo.toml` / `vite.config.ts` / `tauri.conf.json`
- 三平台打包配置

### Definition of Done
- [ ] `cargo test --all-features` 全过（合并逻辑 100% 覆盖）
- [ ] `bun run test` 全过
- [ ] `bun run e2e` 关键场景全过
- [ ] `cargo clippy -- -D warnings` 无警告
- [ ] `bun run lint` 无错误
- [ ] `bun run tauri build` 三平台均能成功打包
- [ ] F1-F4 全部 VERDICT: APPROVE

### Must Have
1. 跨平台桌面应用（macOS/Windows/Linux）
2. 配置 CRUD（每个配置一个 JSON 文件）
3. 「应用」按钮：原子写入 opencode.jsonc + oh-my-openagent.json
4. 自动备份 + 恢复
5. 「✓ 激活」状态提示（启动时检测 + 应用时更新）
6. 「从 opencode 导入」功能（自动填充表单）
7. 深度合并 provider.omos 块（保留 target.model.headers/limit/whitelist 等）
8. 整体替换 oh-my-openagent.json（含 $schema + agents + categories）
9. 导入/导出单配置为 .json
10. 重启 opencode 提示
11. 强制备份（备份失败 → 拒绝应用）
12. 12 个合并边界场景 100% 单元测试
13. 关键 E2E 场景（创建/激活/合并/损坏/备份）

### Must NOT Have（Guardrails）
- ❌ provider key 让用户填（固定为 `omos`，不暴露）
- ❌ 整体覆盖 opencode.jsonc（必须保留 plugin/permission/$schema 等）
- ❌ 强类型反序列化 opencode.jsonc（会丢 headers/limit/whitelist 等未知字段）
- ❌ 跳过备份直接写入（备份失败必须拒绝）
- ❌ 直接 `fs::write`（必须 tempfile + rename 原子操作）
- ❌ 用 apiKey 字符串做激活判定（共用账号场景会乱）
- ❌ flock opencode 配置文件（阻挡 opencode 自身保存）
- ❌ MVP 包含代码签名 / 公证 / 自动更新
- ❌ 多语言（仅中文）
- ❌ 教程 / wizard（首屏空状态引导即可）
- ❌ 自定义 opencode 路径（MVP 不做）

### Spec Framework Integration
未检测到 OpenSpec / Spec Kit 框架，纯自研项目。

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** - 所有验证由 agent 执行，禁止"用户手动测试"。

### Test Decision
- **Infrastructure exists**: NO（绿地项目）
- **Automated tests**: TDD（合并逻辑）+ tests-after（UI）
- **Framework**: Rust 内置 + Playwright + tauri-driver
- **If TDD**: 合并逻辑（merge_provider / merge_models）必须先写失败测试再写实现

### QA Policy
每个 task 必含 agent-executed QA Scenarios。

- **Rust 单元测试**：`cargo test --all-features` → 合并 12 边界 100% 覆盖
- **Rust 集成测试**：`tests/` → 备份、路径、并发
- **E2E（Playwright + tauri-driver）**：
  - 创建配置 → 保存 → 列表显示
  - 激活配置 → 备份存在 → opencode 文件被修改 → 激活徽章显示
  - 合并边界：target.model.headers 被保留
  - 损坏文件：拒绝应用 + 友好错误
  - 导入/导出：JSON 内容一致

### Evidence Policy
- 单元测试：`cargo test` 输出到 terminal
- E2E：截图保存到 `e2e/screenshots/`
- 备份快照：保存到 `e2e/snapshots/`
- 失败场景：错误日志保存到 `e2e/errors/`

---

## Execution Strategy

### Parallel Execution Waves

> 5-8 tasks/wave 是理想（除 final 外 < 3 = 拆分不足）。
> 本计划：Wave 1-4 各 5 个 task，并行度足够。

```
Wave 1 (基础设施 - 5 tasks, 立即启动):
├── T1. Tauri 项目脚手架 + Cargo.toml + 依赖
├── T2. AppError 类型 + 平台路径解析 + 目录初始化
├── T3. JSONC 工具 (jsonc-parser 封装)
├── T4. 配置存储层 (CRUD: list/get/create/update/delete)
└── T5. 激活状态模块 (active.json + 指纹对比)

Wave 2 (核心业务逻辑 - 5 tasks, 依赖 Wave 1):
├── T6. 合并策略 (merge_provider + merge_models 12 边界) [deep, TDD]
├── T7. 备份与原子写入 (tempfile + rename)
├── T8. "从 opencode 导入" 功能
├── T9. Tauri 命令层 (8 commands)
└── T10. Rust 单元测试套件 (12 边界 + 路径 + 备份) [deep]

Wave 3 (前端 UI - 5 tasks, 依赖 Wave 2):
├── T11. 前端脚手架 + Vite + React + TS + Tailwind + 状态管理
├── T12. 列表页 + 路由 + 激活徽章
├── T13. 编辑页表单 (11 agents + 8 categories + models)
├── T14. 导入/导出/备份 UI
└── T15. "从 opencode 导入" UI + 应用按钮 + 重启提示

Wave 4 (质量与打包 - 4 tasks, 依赖 Wave 3):
├── T16. 单实例锁 + tauri-driver + Playwright E2E 框架
├── T17. E2E 关键场景 (创建/激活/合并/损坏/备份/导入导出)
├── T18. Clippy + Lint + CI 配置 + README
└── T19. 三平台打包脚本 (mac/win/linux) + GitHub Actions

Wave FINAL (4 任务并行 + 用户明确同意):
├── F1. Plan compliance audit [oracle]
├── F2. Code quality review [unspecified-high]
├── F3. Real manual QA [unspecified-high]
└── F4. Scope fidelity check [deep]
```

### Dependency Matrix

```
T1  → T2, T3, T4, T5
T2  → T4, T5
T3  → T6, T7, T8
T4  → T5, T9
T5  → T9, T15
T6  → T9, T10
T7  → T8, T9
T8  → T9, T15
T9  → T10, T12, T13, T14, T15
T10 → T17
T11 → T12, T13, T14, T15
T12 → T14, T15
T13 → T14, T15
T14 → T15, T17
T15 → T17
T16 → T17
T17 → T18
T18 → T19
T19 → F1-F4
```

### Agent Dispatch Summary

- **Wave 1**: 5 × `quick`（脚手架 + 基础模块）
- **Wave 2**: 5 × `deep`（合并逻辑 TDD 是核心，必须 deep）
- **Wave 3**: 5 × `visual-engineering`（UI 工作）
- **Wave 4**: 4 × 混合（E2E = deep, 打包 = quick, CI = quick）
- **Wave FINAL**: F1=oracle, F2=unspecified-high, F3=unspecified-high, F4=deep

---

## TODOs

> 实现 + 测试 = 一个 Task。绝不分离。
> 每个 Task 必含：Recommended Agent Profile + Parallelization + QA Scenarios。
> 任务标签用裸数字：`1.` `2.` `3.`，**禁止** `T1.` `Phase 1:` `Task-1.`。
> Final Wave 用 `F1.` `F2.` `F3.` `F4.`，**禁止** `T-F1.` `F-1.` `Final-1.`。

<!-- TASKS_INSERTED_BELOW -->

- [ ] 1. Tauri v2 项目脚手架 + 依赖

  **What to do**:
  - 初始化 Tauri v2 项目：`bun create tauri-app` 选 React + TS + Vite
  - 编辑 `Cargo.toml`：加 `serde`、`serde_json`、`tempfile`、`jsonc-parser`、`dirs`、`uuid`、`chrono`、`thiserror`、`tauri-plugin-dialog`、`tauri-plugin-fs`、`tauri-plugin-single-instance`、`tauri-plugin-updater`
  - 编辑 `tauri.conf.json`：配置 `bundle.targets`（mac: ["dmg", "app"]; win: ["msi", "nsis"]; linux: ["deb", "appimage"]）、`bundle.icon`、productName
  - 写 `.gitignore`：排除 `target/`、`node_modules/`、`dist/`、`.env`
  - 写 `package.json` 脚本：`dev` / `build` / `lint` / `typecheck` / `test` / `e2e` / `tauri`
  - 跑一次 `bun install` 和 `bun run tauri dev` 确认脚手架能启动

  **Must NOT do**:
  - 不引入未列出的重型依赖（如 `tokio` 全功能、`reqwest` 等）
  - 不在脚手架阶段写业务逻辑

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 标准脚手架配置，无复杂逻辑
  - **Skills**: []
    - 理由：脚手架是模板化工作，无需特殊 skill

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T2-T5 平行 — 但 T2-T5 也依赖此）
  - **Parallel Group**: Wave 1 第 1 个（先于 T2-T5 完成）
  - **Blocks**: T2, T3, T4, T5, T9, T10, T11+
  - **Blocked By**: None

  **References**:
  - Tauri v2 官方文档: https://v2.tauri.app/start/create-project/
  - Tauri 配置文件参考: https://v2.tauri.app/reference/config/

  **Acceptance Criteria**:
  - [ ] `bun run tauri dev` 在 macOS 启动成功，显示默认欢迎页
  - [ ] `Cargo.toml` 包含所有声明的依赖
  - [ ] `tauri.conf.json` 含三平台打包 targets
  - [ ] `.gitignore` 完整

  **QA Scenarios**:
  ```
  Scenario: 脚手架启动
    Tool: Bash
    Steps:
      1. cd <project>
      2. bun install
      3. timeout 30 bun run tauri dev 2>&1 | head -50
    Expected: 看到 "compiled successfully" 或 "App listening"
    Evidence: .omo/evidence/task-1-scaffold-startup.txt
  ```

  **Commit**: `chore(scaffold): 初始化 Tauri v2 + 依赖`

---

- [ ] 2. AppError 类型 + 平台路径解析 + 目录初始化

  **What to do**:
  - 写 `src-tauri/src/error.rs`：定义 `AppError` enum（覆盖 11 个边界：FileNotFound / PermissionDenied / InvalidJson / BackupFailed / ActiveConfigMissing / ConfigNotFound / IoError / JsoncParse / OpencodeNotFound / ProviderMismatch / Unknown）
  - 实现 `From<io::Error>` / `From<serde_json::Error>` / `From<jsonc_parser::Error>` 自动转换
  - 实现 `Serialize` for AppError（用 `serde(tag = "kind", content = "details")`）
  - 写 `src-tauri/src/storage/paths.rs`：
    - `pub fn configs_dir() -> Result<PathBuf>` — macOS: `~/Library/Application Support/oh-my-openagent-switch/configs/`, Linux: `${XDG_CONFIG_HOME:-~/.config}/oh-my-openagent-switch/configs/`, Windows: `{FOLDERID_RoamingAppData}/oh-my-openagent-switch/configs/`
    - `pub fn backups_dir() -> Result<PathBuf>` — 同上但 `/backups/`
    - `pub fn opencode_dir() -> Result<PathBuf>` — `~/.config/opencode/`
    - `pub fn active_file() -> Result<PathBuf>` — `.../oh-my-openagent-switch/active.json`
  - 写 `src-tauri/src/storage/init.rs`：`pub fn ensure_dirs() -> Result<()>` — 创建所有目录（递归）
  - 写 `#[cfg(test)]` 单元测试覆盖三平台路径

  **Must NOT do**:
  - 不硬编码 `/Users/...` 等绝对路径
  - 不在 `AppError` 里塞无关数据
  - 不让 `AppError` 实现 `Display` 之外的 `to_string` 重载

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 标准错误类型 + 平台路径查询，模板化
  - **Skills**: []
    - 理由：标准 Rust，无特殊依赖

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T3, T4, T5 平行）
  - **Parallel Group**: Wave 1
  - **Blocks**: T4, T5, T7, T9
  - **Blocked By**: T1

  **References**:
  - Rust `dirs` crate: https://docs.rs/dirs/latest/dirs/
  - Tauri v2 路径 API: https://v2.tauri.app/reference/javascript/api/namespace/path/

  **Acceptance Criteria**:
  - [ ] `cargo build` 成功
  - [ ] `cargo test storage::paths` 全过（覆盖 mac/win/linux 三平台）
  - [ ] `cargo clippy -- -D warnings` 无警告

  **QA Scenarios**:
  ```
  Scenario: 平台路径解析
    Tool: Bash (cargo test)
    Steps:
      1. cd src-tauri
      2. cargo test storage::paths -- --nocapture
    Expected: 3 个 test 全部 PASS（mac/win/linux 各一个）
    Evidence: .omo/evidence/task-2-paths-test.txt

  Scenario: AppError 序列化
    Tool: Bash (cargo test)
    Steps:
      1. cargo test error::tests -- --nocapture
    Expected: 11 个 AppError variant 序列化测试 PASS
    Evidence: .omo/evidence/task-2-error-test.txt
  ```

  **Commit**: `feat(error): AppError + 平台路径解析`

---

- [ ] 3. JSONC 工具（jsonc-parser 封装 + Value 互转）

  **What to do**:
  - 写 `src-tauri/src/config/jsonc.rs`：
    - `pub fn parse_jsonc(s: &str) -> Result<serde_json::Value, AppError>` — 用 `jsonc_parser::parse_to_serde_value`
    - `pub fn strip_jsonc_comments(s: &str) -> Result<String, AppError>` — 用 `jsonc_parser::parse_to_value` + 重新序列化
    - 写 `pub fn pretty_json(v: &serde_json::Value) -> Result<String, AppError>` — `serde_json::to_string_pretty`
  - 写 `#[cfg(test)]` 单元测试：
    - 解析带注释的 JSONC
    - 解析带尾逗号的 JSONC
    - 解析损坏的 JSONC 返回 InvalidJson 错误
    - round-trip：解析后重新序列化保留 value

  **Must NOT do**:
  - 不引入第二个 JSON 解析库
  - 不在 jsonc.rs 里掺业务逻辑

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 单个 crate 的封装 + 测试，模板化
  - **Skills**: []
    - 理由：标准 Rust 测试

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T2, T4, T5 平行）
  - **Parallel Group**: Wave 1
  - **Blocks**: T6, T7, T8
  - **Blocked By**: T1

  **References**:
  - jsonc-parser crate: https://docs.rs/jsonc-parser/latest/jsonc_parser/
  - serde_json::Value: https://docs.rs/serde_json/latest/serde_json/enum.Value.html

  **Acceptance Criteria**:
  - [ ] `cargo test config::jsonc` 全过
  - [ ] `cargo build` 成功

  **QA Scenarios**:
  ```
  Scenario: JSONC 解析保留 value
    Tool: Bash
    Steps:
      1. cd src-tauri
      2. cargo test config::jsonc -- --nocapture
    Expected: 4 个测试全 PASS（注释、尾逗号、损坏、round-trip）
    Evidence: .omo/evidence/task-3-jsonc-test.txt
  ```

  **Commit**: `feat(jsonc): 解析 + 序列化封装`

---

- [ ] 4. 配置存储层（CRUD: list/get/create/update/delete）

  **What to do**:
  - 定义数据结构 `pub struct Config { id: String, label: String, created_at: DateTime<Utc>, updated_at: DateTime<Utc>, payload: ConfigPayload }`（其中 `ConfigPayload` 是项目说明里"表单"对应的 JSON 形状，**待 T6 完成时定**）
  - 写 `src-tauri/src/storage/configs.rs`：
    - `pub fn list_configs() -> Result<Vec<ConfigMeta>, AppError>` — 扫 `configs/*.json`，返回 `(id, label, updated_at)` 列表
    - `pub fn get_config(id: &str) -> Result<Config, AppError>` — 读单文件
    - `pub fn create_config(label: &str) -> Result<Config, AppError>` — 生成 UUID + 写空 ConfigPayload
    - `pub fn update_config(id: &str, payload: ConfigPayload) -> Result<Config, AppError>` — 更新 `updated_at` + 写回
    - `pub fn delete_config(id: &str) -> Result<(), AppError>` — 删除文件
  - 文件名 = `{uuid}.json`，UUID 用 v4
  - 写 8 个 `#[cfg(test)]` 单元测试（用 `tempfile` 隔离）

  **Must NOT do**:
  - 不在 configs.rs 里定义 `ConfigPayload` 形状（放 T6）
  - 不在 CRUD 里做备份逻辑
  - 不在 delete 时连带删除 active.json（这是 T5 的事）

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 标准 CRUD + 文件操作
  - **Skills**: []
    - 理由：模板化

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T2, T3, T5 平行）
  - **Parallel Group**: Wave 1
  - **Blocks**: T9
  - **Blocked By**: T1, T2

  **References**:
  - uuid crate: https://docs.rs/uuid/latest/uuid/
  - chrono crate: https://docs.rs/chrono/latest/chrono/
  - tempfile crate（测试用）: https://docs.rs/tempfile/latest/tempfile/

  **Acceptance Criteria**:
  - [ ] `cargo test storage::configs` 全过（8 测试）
  - [ ] `cargo clippy -- -D warnings` 无警告

  **QA Scenarios**:
  ```
  Scenario: CRUD 完整流程
    Tool: Bash
    Steps:
      1. cd src-tauri
      2. cargo test storage::configs -- --nocapture
    Expected: 8 个测试全 PASS
    Evidence: .omo/evidence/task-4-configs-test.txt
  ```

  **Commit**: `feat(storage): 配置 CRUD`

---

- [ ] 5. 激活状态模块（active.json + 指纹对比）

  **What to do**:
  - 写 `src-tauri/src/storage/active.rs`：
    - `pub struct ActiveRecord { config_id: String, applied_at: DateTime<Utc>, fingerprints: Fingerprints }`
    - `pub struct Fingerprints { opencode: String, omos: String }`（SHA256 字符串）
    - `pub fn read_active() -> Result<Option<ActiveRecord>, AppError>` — 读 `active.json`，文件不存在返回 `Ok(None)`
    - `pub fn write_active(record: &ActiveRecord) -> Result<(), AppError>` — 原子写入（tempfile + rename）
    - `pub fn clear_active() -> Result<(), AppError>` — 删除文件
  - 写 `src-tauri/src/config/fingerprint.rs`：
    - `pub fn fingerprint(value: &serde_json::Value) -> String` — 规范化后 SHA256（去掉空白、统一字段顺序）
  - 写 `src-tauri/src/storage/detect.rs`（启动时调用）：
    - `pub fn detect_active(configs: &[ConfigMeta]) -> Result<ActiveStatus, AppError>`
    - 4 种状态：`Active` / `Drifted`（fingerprint 不匹配）/ `Unknown`（active.json 缺失或损坏）/ `Orphan`（引用的 configId 找不到）
  - 写 6 个 `#[cfg(test)]` 单元测试

  **Must NOT do**:
  - 不在 fingerprint 里用 MD5（必须 SHA256）
  - 不在 active.json 里包含 apiKey（**只放 fingerprint**）
  - 不在 detect 里返回 `Result<Option<...>>`（用枚举）

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 标准状态管理 + 哈希
  - **Skills**: []
    - 理由：模板化

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T2, T3, T4 平行）
  - **Parallel Group**: Wave 1
  - **Blocks**: T9, T15
  - **Blocked By**: T1, T2, T3

  **References**:
  - sha2 crate: https://docs.rs/sha2/latest/sha2/
  - serde tagged enum: https://serde.rs/enum-representations.html

  **Acceptance Criteria**:
  - [ ] `cargo test storage::active config::fingerprint storage::detect` 全过
  - [ ] active.json 写入后能正确读回
  - [ ] fingerprint 对相同 Value 产生相同 hash（顺序无关）

  **QA Scenarios**:
  ```
  Scenario: 4 种激活状态检测
    Tool: Bash
    Steps:
      1. cd src-tauri
      2. cargo test storage::detect -- --nocapture
    Expected: 4 个测试 PASS（Active / Drifted / Unknown / Orphan）
    Evidence: .omo/evidence/task-5-active-detect-test.txt
  ```

  **Commit**: `feat(active): 激活状态检测`

---

- [ ] 6. 合并策略（merge_provider + merge_models 12 边界） [TDD, deep]

  **What to do**:
  - **TDD 流程**：先写 12 个失败测试，再写实现
  - 写 `src-tauri/src/config/merge.rs`：
    - `pub struct ConfigPayload { label: String, provider: ProviderConfig, agents: HashMap<String, String>, categories: HashMap<String, String> }`
    - `pub struct ProviderConfig { name: String, npm: String, options: ProviderOptions, models: HashMap<String, ModelEntry> }`
    - `pub struct ModelEntry { name: String, group: Option<String> }`
    - `pub fn merge_opencode(target: &mut serde_json::Value, source: &ConfigPayload) -> Result<(), AppError>` — 深度合并 `provider.omos` 块
    - `pub fn build_oh_my_openagent(payload: &ConfigPayload) -> serde_json::Value` — 构建整体替换文件
  - 合并规则（**关键**）：
    - `target.plugin` / `target.permission` / `target.$schema` / `target.provider.<other>` **完全保留**
    - `target.provider.omos.name` / `target.provider.omos.npm` / `target.provider.omos.options` 整体替换
    - `target.provider.omos.models`：**深度合并**：
      - 相同 model id：只覆盖 `name` 和 `group` 字段，**保留** `headers` / `limit` / `whitelist` / `attachment` / `cost` / `modalities` / `experimental` / `provider` 等
      - 新增 model id：整体加入
      - 默认**不删除**源中没有的 model（保守策略；UI 后续加 checkbox）
  - 写 12 个 `#[cfg(test)]` 单元测试（见下面 QA Scenarios）

  **Must NOT do**:
  - 不在 merge 里用强类型反序列化（用 `serde_json::Value`）
  - 不在 merge 里写 active.json（那是 T5/T7）
  - 不在 merge 里触碰 backups（那是 T7）
  - 不删除 target.model.headers 等未知字段

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 核心业务逻辑，12 边界复杂
  - **Skills**: [`test-driven-development`]
    - 理由：必须 TDD 流程

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T7, T8 平行）
  - **Parallel Group**: Wave 2
  - **Blocks**: T9, T10, T17
  - **Blocked By**: T1, T3

  **References**:
  - serde_json::Value API: https://docs.rs/serde_json/latest/serde_json/enum.Value.html
  - opencode config.json schema: https://opencode.ai/config.json
  - oh-my-openagent 配置文档: https://github.com/code-yeongyu/oh-my-openagent/blob/dev/docs/reference/configuration.md

  **Acceptance Criteria**:
  - [ ] 12 个 merge 单元测试全过
  - [ ] `cargo test` 全过
  - [ ] `cargo clippy -- -D warnings` 无警告

  **QA Scenarios** (12 个必测边界):
  ```
  Scenario 1: target.model.headers 保留
    Setup: target = { models: { m1: { name: "old", headers: { "X-Custom": "x" } } } }
           source.models = { m1: { name: "new" } }
    Expected: 合并后 m1.name = "new", m1.headers = { "X-Custom": "x" }
  Scenario 2: target.model.limit 保留
    Setup: target.m1 = { name: "old", limit: { context: 100000 } }
           source.m1 = { name: "new" }
    Expected: m1.limit = { context: 100000 } 保留
  Scenario 3: target.model.group 保留
    Setup: target.m1 = { name: "old", group: "default" }
           source.m1 = { name: "new" }
    Expected: m1.group = "default" 保留
  Scenario 4: 源中删除的 model 在目标中保留（默认保守）
    Setup: target = { m1: ..., m2: ... }, source = { m1: ... }
    Expected: 合并后 m1 和 m2 都在
  Scenario 5: target.options 整块替换（apiKey / baseURL 整体替换，不合并内部字段——安全考虑）
    Setup: target.options = { apiKey: "old", timeout: 600 }, source.options = { apiKey: "new" }
    Expected: 整块替换为 source.options；timeout 字段丢失（这是**预期行为**，不是 bug）
  Scenario 6: target.provider 含 anthropic → 保留
    Setup: target = { provider: { omos: {...}, anthropic: {...} } }, source.provider.omos
    Expected: 合并后 omos 替换, anthropic 保留
  Scenario 7: target 含 $schema → 保留
    Setup: target = { $schema: "https://...", provider: { omos: ... } }
    Expected: $schema 字段保留
  Scenario 8: target 含 plugin → 保留
    Setup: target = { plugin: ["oh-my-openagent"], provider: { omos: ... } }
    Expected: plugin 字段保留
  Scenario 9: target 含 permission → 保留
    Setup: target = { permission: {...}, provider: { omos: ... } }
    Expected: permission 整块保留
  Scenario 10: source 与 target 完全相同
    Expected: 不产生无意义修改（值不变）
  Scenario 11: 源中新增 model
    Setup: target.models = {}, source.models = { m1: { name: "..." } }
    Expected: target.models.m1 被新增
  Scenario 12: target 中 model 的未知字段 (experimental, modalities, attachment) 保留
    Setup: target.m1 = { name: "old", experimental: { x: 1 }, modalities: {...} }
           source.m1 = { name: "new" }
    Expected: experimental 和 modalities 都保留

  Scenario: 全部 12 个
    Tool: Bash
    Steps:
      1. cd src-tauri
      2. cargo test config::merge -- --nocapture
    Expected: 12 passed; 0 failed
    Evidence: .omo/evidence/task-6-merge-test.txt
  ```

  **Commit**: `feat(merge): 深度合并 provider 块 (TDD)`

---

- [ ] 7. 备份与原子写入（tempfile + rename）

  **What to do**:
  - 写 `src-tauri/src/storage/backup.rs`：
    - `pub fn backup_file(path: &Path) -> Result<PathBuf, AppError>` — 复制到 `backups/{basename}-{timestamp}.json`，时间戳格式 `2026-06-19T22-30-00`
    - `pub fn list_backups() -> Result<Vec<BackupMeta>, AppError>` — 扫 `backups/`，返回 `(filename, original_path, created_at, size_bytes)`
    - `pub fn restore_backup(backup_filename: &str) -> Result<(), AppError>` — 从备份恢复到原位置
    - `pub fn prune_old_backups(keep: usize) -> Result<(), AppError>` — 只保留最近 N 个备份（默认 30）
  - 写 `src-tauri/src/storage/atomic.rs`：
    - `pub fn atomic_write(path: &Path, content: &str) -> Result<(), AppError>` — 写 `.tmp` → `rename(2)` / `MoveFileExW`
    - `pub fn atomic_write_json(path: &Path, value: &serde_json::Value) -> Result<(), AppError>` — 同上但 value 先序列化
  - 写 8 个 `#[cfg(test)]` 单元测试：
    - 备份/恢复 round-trip
    - 时间戳格式正确
    - 原子写入在断电时不会留半成品（模拟 rename 失败）
    - 备份不存在时返回 BackupNotFound
    - prune 保留最新的 N 个
    - 备份权限继承
    - 同名备份滚动覆盖（按时间戳不会冲突，但并发场景要测）
    - 备份目录不存在时自动创建

  **Must NOT do**:
  - 不在 atomic_write 里做备份（备份是单独调用）
  - 不在 backup.rs 里调 atomic_write
  - 不让 backup 文件名含空格

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 原子操作细节多，跨平台有差异
  - **Skills**: []
    - 理由：Rust 文件系统操作标准库

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T6, T8 平行）
  - **Parallel Group**: Wave 2
  - **Blocks**: T8, T9
  - **Blocked By**: T1, T2

  **References**:
  - tempfile crate: https://docs.rs/tempfile/latest/tempfile/
  - Rust fs::rename: https://doc.rust-lang.org/std/fs/fn.rename.html
  - Windows MoveFileEx: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw

  **Acceptance Criteria**:
  - [ ] 8 个 backup/atomic 测试全过
  - [ ] 跨平台测试通过（mac/win/linux 各跑一次）

  **QA Scenarios**:
  ```
  Scenario: 备份 round-trip
    Tool: Bash
    Steps:
      1. cd src-tauri
      2. cargo test storage::backup storage::atomic -- --nocapture
    Expected: 8 passed
    Evidence: .omo/evidence/task-7-backup-test.txt

  Scenario: 跨平台
    Tool: Bash
    Steps:
      1. mac: cargo test --target aarch64-apple-darwin
      2. linux: cargo test --target x86_64-unknown-linux-gnu (如可用)
      3. win: cargo test --target x86_64-pc-windows-msvc (如可用)
    Expected: 都过
    Evidence: .omo/evidence/task-7-backup-cross-platform.txt
  ```

  **Commit**: `feat(backup): 备份与原子写入`

---

- [ ] 8. 「从 opencode 导入」功能

  **What to do**:
  - 写 `src-tauri/src/storage/import.rs`：
    - `pub fn read_current_opencode() -> Result<Option<ProviderConfig>, AppError>` — 读 `~/.config/opencode/opencode.jsonc`，提取 `provider.omos` 块；不存在返回 `Ok(None)`
    - 转换规则：把 `provider.omos` 的 `Value` 转成 `ProviderConfig`，**只提取** `name` / `npm` / `options` / `models`，**忽略**其他字段
    - 对每个 model：提取 `name` 必有，`group` 可选，其他字段不读
  - 写 `src-tauri/src/storage/export.rs`：
    - `pub fn export_config(id: &str, target: &Path) -> Result<(), AppError>` — 复制 config 文件到 `target` 路径（按 `target` 直接复制，不改名）
    - `pub fn import_config(source: &Path) -> Result<Config, AppError>` — 读 `source` JSON，校验基本字段，生成新 UUID，写入 configs/
  - 写 5 个 `#[cfg(test)]` 单元测试：
    - opencode.jsonc 不存在 → 返回 `Ok(None)`
    - opencode.jsonc 没有 provider.omos → 返回 `Ok(None)`
    - opencode.jsonc 含 provider.omos 但 models 为空 → 返回 `Ok(Some(empty_provider))`
    - 导入文件 schema 不符 → 返回 `InvalidJson` 错误
    - 导入成功 → 生成新 UUID + 写入

  **Must NOT do**:
  - 不在 import 里直接修改 opencode 文件（只读）
  - 不在 export 里包含 active.json 内容
  - 不在 import 时覆盖已有同名 config

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 简单的读写 + 转换
  - **Skills**: []
    - 理由：模板化

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T6, T7 平行）
  - **Parallel Group**: Wave 2
  - **Blocks**: T9, T15
  - **Blocked By**: T3, T7

  **References**:
  - 项目说明实际配置: `/Users/zaxcler/.config/opencode/opencode.jsonc`
  - serde_json Value 转换: https://docs.rs/serde_json/latest/serde_json/enum.Value.html

  **Acceptance Criteria**:
  - [ ] 5 个 import/export 测试全过
  - [ ] 从实际 `~/.config/opencode/opencode.jsonc` 导入成功（在测试 fixture 中模拟）

  **QA Scenarios**:
  ```
  Scenario: 从真实 opencode 配置导入
    Tool: Bash
    Steps:
      1. cd src-tauri
      2. cargo test storage::import storage::export -- --nocapture
    Expected: 5 passed
    Evidence: .omo/evidence/task-8-import-test.txt
  ```

  **Commit**: `feat(import): 从 opencode 导入 + 单文件导入导出`

---

- [ ] 9. Tauri 命令层（12 commands）

  **What to do**:
  - 写 `src-tauri/src/commands/mod.rs` 暴露 12 个 `#[tauri::command]`：
    1. `list_configs() -> Result<Vec<ConfigMeta>>` — 调 T4
    2. `get_config(id: String) -> Result<Config>` — 调 T4
    3. `create_config(label: String) -> Result<Config>` — 调 T4
    4. `update_config(id: String, payload: ConfigPayload) -> Result<Config>` — 调 T4
    5. `delete_config(id: String) -> Result<()>` — 调 T4
    6. `apply_config(id: String) -> Result<ApplyResult>` — 调 T7 (备份) → T6 (合并) → T7 (原子写) → T5 (写 active.json) → 返回 `ApplyResult { backup_files: Vec<PathBuf>, applied_at: DateTime<Utc> }`
    7. `import_from_opencode() -> Result<Option<Config>>` — 调 T8 + T4 (保存为新 config)
    8. `import_config_file(path: String) -> Result<Config>` — 调 T8
    9. `export_config(id: String, target: String) -> Result<()>` — 调 T8
    10. `list_backups() -> Result<Vec<BackupMeta>>` — 调 T7
    11. `restore_backup(filename: String) -> Result<()>` — 调 T7
    12. `get_active_status() -> Result<ActiveStatus>` — 调 T5
  - 写 `src-tauri/src/lib.rs` 注册所有命令
  - 写 4 个 `#[cfg(test)]` 集成测试（用 `tauri::test` MockRuntime）

  **Must NOT do**:
  - 不在 commands 里写业务逻辑（只调模块）
  - 不在 commands 里返回 `Result<T, String>`（必须 `AppError`）
  - 不让 `apply_config` 跳过备份（必须强制）

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 标准 Tauri command 包装
  - **Skills**: []
    - 理由：模板化

  **Parallelization**:
  - **Can Run In Parallel**: NO（必须等 T6/T7/T8 完成）
  - **Parallel Group**: Wave 2 最后
  - **Blocks**: T11+ (前端)
  - **Blocked By**: T4, T5, T6, T7, T8

  **References**:
  - Tauri v2 Commands: https://v2.tauri.app/develop/calling-rust/
  - Tauri v2 测试: https://v2.tauri.app/develop/tests/

  **Acceptance Criteria**:
  - [ ] 12 个 commands 编译通过
  - [ ] 4 个集成测试全过
  - [ ] `cargo tauri dev` 启动后 `window.__TAURI_INTERNALS__` 能 invoke 命令

  **QA Scenarios**:
  ```
  Scenario: 命令层集成测试
    Tool: Bash
    Steps:
      1. cd src-tauri
      2. cargo test commands -- --nocapture
    Expected: 4 passed
    Evidence: .omo/evidence/task-9-commands-test.txt
  ```

  **Commit**: `feat(commands): Tauri 命令层 (12 commands)`

---

- [ ] 10. Rust 单元测试套件（覆盖 12 边界 + 路径 + 备份 + CRUD）

  **What to do**:
  - 跑全套 `cargo test --all-features` 确保所有 Wave 1-2 测试都过
  - 添加 `tests/integration_test.rs` 跨模块集成测试：
    - "创建 config → 激活 → 验证 opencode 文件被修改 → 验证 active.json 更新 → 验证 backup 存在"
    - "合并模式：target.model.headers 保留"
    - "激活检测：Drifted 状态正确"
    - "备份恢复：恢复后 opencode 文件与备份一致"
  - 写 `.cargo/config.toml` 加 test runner 配置
  - 在 `Cargo.toml` 加 `[dev-dependencies]`：`tempfile = "3"`, `mockall = "0.13"`（如需 mock）

  **Must NOT do**:
  - 不修改 Wave 1-2 已有的测试（保证基线不退化）
  - 不让 `cargo test` 依赖外部文件

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 跨模块集成测试，需要理解所有 Wave 1-2 模块
  - **Skills**: [`test-driven-development`]
    - 理由：必须保持测试质量

  **Parallelization**:
  - **Can Run In Parallel**: NO（必须等所有 Wave 2 单元测试）
  - **Parallel Group**: Wave 2 收尾
  - **Blocks**: T17
  - **Blocked By**: T6, T7, T8, T9

  **References**:
  - Rust 集成测试: https://doc.rust-lang.org/book/ch11-03-test-organization.html
  - Tauri MockRuntime: https://v2.tauri.app/develop/tests/

  **Acceptance Criteria**:
  - [ ] `cargo test --all-features` 全过
  - [ ] 集成测试覆盖 5 个关键场景
  - [ ] 测试覆盖率（用 `cargo tarpaulin`）：合并逻辑 100%，路径 100%，备份 100%

  **QA Scenarios**:
  ```
  Scenario: 全套 Rust 测试
    Tool: Bash
    Steps:
      1. cd src-tauri
      2. cargo test --all-features -- --nocapture
    Expected: 全部 PASS；0 failed
    Evidence: .omo/evidence/task-10-full-test.txt

  Scenario: 覆盖率
    Tool: Bash
    Steps:
      1. cargo install cargo-tarpaulin (如未装)
      2. cargo tarpaulin --out Xml
    Expected: merge 100%, paths 100%, backup 100%, configs 100%
    Evidence: .omo/evidence/task-10-coverage.xml
  ```

  **Commit**: `test(integration): 跨模块集成测试`

---

- [ ] 11. 前端脚手架 + Vite + React + TS + Tailwind + 状态管理

  **What to do**:
  - 完善 Vite + React + TS 配置：`vite.config.ts`（加 `@` 路径别名指向 `src/`）
  - 装 Tailwind v4 + PostCSS
  - 装 `zustand` 状态管理
  - 装 `react-router-dom` v6
  - 装 `react-hook-form` + `zod` 表单处理
  - 写 `src/lib/tauri.ts`：包装 `invoke<T>()` + 自动加错误处理
  - 写 `src/types/index.ts`：从 `src-tauri/src/config/merge.rs` 复制类型到 TS（手写，不用 codegen）
  - 写 `src/store/configs.ts`：Zustand store（configs 列表、activeStatus、loading 状态）
  - 写 `src/components/Layout.tsx`：顶部（左侧 app 名 + 右侧 + 按钮）+ 主体内容区
  - 写 `src/lib/constants.ts`：列出 11 个 agent key + 8 个 category key + 显示名映射

  **Must NOT do**:
  - 不引入未列出的 UI 库（不要 antd / mui）
  - 不让前端直接调文件系统（必须通过 Tauri commands）
  - 不在 types/index.ts 里重复定义 ConfigPayload（必须与 Rust 端一致）

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: UI 基础架构
  - **Skills**: [`frontend-ui-ux`]
    - 理由：UI 设计 + Tailwind 配置

  **Parallelization**:
  - **Can Run In Parallel**: NO（前端所有 task 的基础）
  - **Parallel Group**: Wave 3 第一个
  - **Blocks**: T12, T13, T14, T15
  - **Blocked By**: T1, T9

  **References**:
  - Tauri v2 JS API: https://v2.tauri.app/reference/javascript/
  - Zustand: https://github.com/pmndrs/zustand
  - react-hook-form: https://react-hook-form.com/
  - Tailwind v4: https://tailwindcss.com/docs

  **Acceptance Criteria**:
  - [ ] `bun run dev` 启动 Vite dev server 成功
  - [ ] `bun run typecheck` 无错
  - [ ] `bun run lint` 无错
  - [ ] `src/lib/tauri.ts` 的 `invoke` 包装有类型签名
  - [ ] `src/types/index.ts` 与 Rust ConfigPayload 字段一致

  **QA Scenarios**:
  ```
  Scenario: 前端启动
    Tool: Bash
    Steps:
      1. cd <project>
      2. bun install
      3. timeout 15 bun run dev 2>&1 | head -30
    Expected: "Local: http://localhost:1420/" 出现
    Evidence: .omo/evidence/task-11-frontend-startup.txt

  Scenario: TypeScript 检查
    Tool: Bash
    Steps:
      1. bun run typecheck
    Expected: 0 errors
    Evidence: .omo/evidence/task-11-typecheck.txt
  ```

  **Commit**: `feat(ui): 前端脚手架 + 状态管理`

---

- [ ] 12. 列表页 + 路由 + 激活徽章

  **What to do**:
  - 写 `src/pages/ListPage.tsx`：
    - 顶部：左侧 "OH-MY-OPENAGENT-SWITCH" + 右侧 "+ 新建" 按钮
    - 主体：配置列表（卡片布局，每项含 label + 更新时间 + 激活状态徽章 + 4 个操作按钮：编辑/应用/导出/删除）
    - 空状态：居中显示 "还没有配置，点 + 新建第一个" + 「从 opencode 导入」按钮
    - 激活徽章：4 种显示（✓ 激活 / ⚠ 已偏离 / ❓ 未知 / — 未激活）
  - 写 `src/router.tsx`：`/` 列表页，`/edit/:id` 编辑页，`/edit/new` 新建页
  - 写 `src/components/ConfirmDialog.tsx`：通用确认对话框（删除/恢复备份时用）
  - 写 `src/components/Toast.tsx`：成功/错误提示

  **Must NOT do**:
  - 不在列表页加搜索/排序（MVP 不做）
  - 不在列表页显示 apiKey
  - 不在删除时不确认

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 列表 UI
  - **Skills**: [`frontend-ui-ux`]
    - 理由：UI 布局

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T13, T14 平行）
  - **Parallel Group**: Wave 3
  - **Blocks**: T17
  - **Blocked By**: T11

  **References**:
  - 项目说明 UI 描述: `项目说明.md:5-9`
  - react-router-dom v6: https://reactrouter.com/en/main

  **Acceptance Criteria**:
  - [ ] 列表页能展示 0/N 个配置
  - [ ] 激活徽章正确显示 4 种状态
  - [ ] 点 "+" 跳转到 `/edit/new`
  - [ ] 点 "编辑" 跳转到 `/edit/:id`

  **QA Scenarios**:
  ```
  Scenario: 列表页渲染
    Tool: Playwright (E2E)
    Steps:
      1. 启动 Tauri dev
      2. 访问 tauri://localhost
      3. 截图列表页空状态
    Expected: 看到 "OH-MY-OPENAGENT-SWITCH" 标题 + "+" 按钮 + "从 opencode 导入" 按钮
    Evidence: .omo/evidence/task-12-list-empty.png

  Scenario: 列表展示多 config
    Tool: Playwright
    Steps:
      1. 创建 2 个 config（mock backend）
      2. 刷新列表
    Expected: 看到 2 个卡片
    Evidence: .omo/evidence/task-12-list-with-configs.png
  ```

  **Commit**: `feat(ui): 列表页 + 路由 + 激活徽章`

---

- [ ] 13. 编辑页表单（11 agents + 8 categories + models）

  **What to do**:
  - 写 `src/pages/EditPage.tsx`：
    - 标题：根据 `id` 参数显示 "新建配置" 或 "编辑：{label}"
    - **基础字段**：label（必填 input）
    - **供应商设置 section**：
      - name（input）
      - npm（input，默认 `@ai-sdk/openai-compatible`）
      - options.apiKey（password input）
      - options.baseURL（input）
      - models（动态数组）：每行 model.id（input）+ model.name（input）+ model.group（可选 input）+ 删除按钮，底部 "+ 添加 model" 按钮
    - **角色模型设置 section**：
      - 11 个 agent（按 constants.ts 顺序）：每个是 select，options = models 列表（值格式 `omos/${model.id}`）
      - 8 个 category（同上）
    - 底部：「保存」按钮 + 「取消」返回列表
  - 用 `react-hook-form` 管理表单状态
  - 用 zod schema 校验（label 必填、apiKey 必填、至少 1 个 model）
  - 写 `src/components/ModelRow.tsx`：单行 model 编辑组件
  - 写 `src/components/RoleSelect.tsx`：单个 role 的 select 组件

  **Must NOT do**:
  - 不让用户填 provider key（不显示，固定 `omos`）
  - 不显示 $schema 字段
  - 不让 role model 留空（默认 models[0]）

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 复杂表单 UI
  - **Skills**: [`frontend-ui-ux`]
    - 理由：表单设计

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T12, T14 平行）
  - **Parallel Group**: Wave 3
  - **Blocks**: T17
  - **Blocked By**: T11

  **References**:
  - 项目说明表单: `项目说明.md:14-67`
  - react-hook-form useFieldArray: https://react-hook-form.com/api/usefieldarray
  - zod schema: https://zod.dev/

  **Acceptance Criteria**:
  - [ ] 加载 `/edit/new` 显示空表单
  - [ ] 加载 `/edit/:id` 显示已有数据
  - [ ] 必填校验：label / apiKey / models[0] 为空时「保存」按钮 disabled
  - [ ] 添加/删除 model 行可用
  - [ ] 保存后跳转回列表

  **QA Scenarios**:
  ```
  Scenario: 新建配置流程
    Tool: Playwright
    Steps:
      1. 列表页点 "+"
      2. 填 label = "Test"
      3. 填 apiKey = "sk-test-xxx"
      4. 填 baseURL = "https://api.test.com/v1"
      5. 填第一个 model.id = "test-m1"、name = "Test Model 1"
      6. 11 个 agent + 8 category 的 select 都有默认选项
      7. 点保存
    Expected: 跳回列表，看到 "Test" 卡片
    Evidence: .omo/evidence/task-13-create-config.png

  Scenario: 校验
    Tool: Playwright
    Steps:
      1. 新建页不填 label
      2. 看保存按钮
    Expected: 保存按钮 disabled
    Evidence: .omo/evidence/task-13-validation.png
  ```

  **Commit**: `feat(ui): 编辑页表单`

---

- [ ] 14. 导入/导出/备份 UI

  **What to do**:
  - 写 `src/components/ImportExportMenu.tsx`：列表项上的「导入」和「导出」按钮
  - 写 `src/pages/BackupsPage.tsx`：
    - 列出所有备份（filename + 原始文件 + 时间 + 大小）
    - 每行「恢复」按钮 + 「删除」按钮
    - 顶部「← 返回列表」
  - 写 `src/components/ApplyResultDialog.tsx`：应用成功后展示
    - 显示「应用成功」
    - 显示备份文件路径列表
    - 「重启 opencode 提示」+ 「复制重启命令」按钮（`pkill -f opencode && opencode`）
  - 在 `src/router.tsx` 加 `/backups` 路由
  - 在列表页加「备份」入口按钮

  **Must NOT do**:
  - 不在备份页显示备份内容（避免误编辑）
  - 不让「恢复」按钮不确认就执行
  - 不在导出时把 active 信息一并导出

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 多个 UI 组件
  - **Skills**: [`frontend-ui-ux`]
    - 理由：UI 设计

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T12, T13 平行）
  - **Parallel Group**: Wave 3
  - **Blocks**: T17
  - **Blocked By**: T11, T12

  **References**:
  - Tauri dialog plugin: https://v2.tauri.app/plugin/dialog/
  - 项目说明导入/导出: 用户确认需要

  **Acceptance Criteria**:
  - [ ] 「导出」按钮触发下载对话框
  - [ ] 导出文件可被「导入」按钮读回
  - [ ] 备份页列出所有备份并能恢复

  **QA Scenarios**:
  ```
  Scenario: 导出导入 round-trip
    Tool: Playwright
    Steps:
      1. 列表中点「导出」→ 选 /tmp/test-config.json
      2. 删除该 config
      3. 点「导入」→ 选 /tmp/test-config.json
    Expected: 列表中重新出现同名 config，内容一致
    Evidence: .omo/evidence/task-14-export-import.png

  Scenario: 备份列表 + 恢复
    Tool: Playwright
    Steps:
      1. 应用一个 config
      2. 进入 /backups
    Expected: 看到刚才生成的备份项
    3. 点「恢复」
    Expected: opencode 文件被恢复
    Evidence: .omo/evidence/task-14-backup-restore.png
  ```

  **Commit**: `feat(ui): 导入/导出/备份 UI`

---

- [ ] 15. 「从 opencode 导入」UI + 应用按钮 + 重启提示

  **What to do**:
  - 写 `src/components/ImportFromOpencodeButton.tsx`：列表页空状态「从 opencode 导入」按钮
  - 写 `src/components/ApplyButton.tsx`：列表项的「应用」按钮
  - 写 `src/components/RestartPrompt.tsx`：应用成功后的对话框
    - 显示「应用成功」+ 「备份位置」
    - 大字提示「需要重启 opencode 才生效」
    - 按钮：「复制重启命令」「知道了」
  - 写 `src/lib/restart-cmd.ts`：
    - macOS/Linux: `pkill -f opencode; opencode`（或类似）
    - Windows: `taskkill /IM opencode.exe /F && start opencode`
  - 写 `src/lib/process-check.ts`（可选，best-effort）：
    - macOS/Linux: `pgrep -f opencode`
    - Windows: `tasklist /FI "IMAGENAME eq opencode.exe"`
    - 返回 `Promise<boolean>`（仅 best-effort 提示，不阻塞）

  **Must NOT do**:
  - 不在应用前做"等待 opencode 退出"逻辑
  - 不强制阻塞
  - 不在重启命令中执行 sudo

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 多个 UI 组件 + 简单命令
  - **Skills**: [`frontend-ui-ux`]
    - 理由：UI 设计

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T12-T14 平行，但必须最后）
  - **Parallel Group**: Wave 3
  - **Blocks**: T17
  - **Blocked By**: T5, T8, T11, T12

  **References**:
  - opencode 重启机制: 调研报告 bg_0f9dbf63
  - 项目说明应用按钮: 用户确认

  **Acceptance Criteria**:
  - [ ] 「从 opencode 导入」点击后跳转编辑页且表单已填
  - [ ] 「应用」按钮调用 `apply_config` 并展示结果对话框
  - [ ] 重启命令在 macOS 复制到剪贴板验证可用

  **QA Scenarios**:
  ```
  Scenario: 从 opencode 导入
    Tool: Playwright
    Steps:
      1. 准备一个真实的 opencode.jsonc (有 provider.omos)
      2. 列表页空状态点「从 opencode 导入」
    Expected: 跳转到编辑页，表单已填充 name/apiKey/baseURL/models
    Evidence: .omo/evidence/task-15-import-from-opencode.png

  Scenario: 应用配置 + 重启提示
    Tool: Playwright
    Steps:
      1. 编辑一个 config 后保存
      2. 列表点「应用」
      3. 看弹窗
    Expected: 看到「应用成功」+ 备份路径 + 「需要重启 opencode」+ 「复制重启命令」按钮
    Evidence: .omo/evidence/task-15-apply-result.png
  ```

  **Commit**: `feat(ui): 从 opencode 导入 + 应用 + 重启提示`

---

- [ ] 16. 单实例锁 + tauri-driver + Playwright E2E 框架

  **What to do**:
  - 在 `src-tauri/Cargo.toml` 加 `tauri-plugin-single-instance`
  - 在 `src-tauri/src/lib.rs` 注册单实例插件
  - 在 `package.json` 加 devDependencies：`@playwright/test`、`@tauri-apps/cli`、`tauri-driver`
  - 写 `playwright.config.ts`：配置 tauri-driver
  - 写 `e2e/fixtures/`：准备测试用 opencode.jsonc 样本（含 provider.omos、有/无 headers）
  - 写 `e2e/helpers/tauri.ts`：包装 tauri-driver 启动/停止

  **Must NOT do**:
  - 不让单实例锁阻止二次启动（应该聚焦到现有窗口）
  - 不在 E2E 用真实 `~/.config/opencode/`（用临时目录覆盖）

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 标准 E2E 框架配置
  - **Skills**: []
    - 理由：模板化

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T18 平行）
  - **Parallel Group**: Wave 4 第一个
  - **Blocks**: T17
  - **Blocked By**: T1, T15

  **References**:
  - tauri-plugin-single-instance: https://v2.tauri.app/plugin/single-instance/
  - tauri-driver: https://v2.tauri.app/develop/tests/webdriver/
  - Playwright: https://playwright.dev/

  **Acceptance Criteria**:
  - [ ] `bun run e2e --list` 能列出测试
  - [ ] 单实例锁编译通过

  **QA Scenarios**:
  ```
  Scenario: E2E 框架就绪
    Tool: Bash
    Steps:
      1. bun run e2e --list
    Expected: 列出 0 个测试（框架就绪但无 case，T17 补）
    Evidence: .omo/evidence/task-16-e2e-ready.txt
  ```

  **Commit**: `chore(e2e): Playwright + tauri-driver 配置`

---

- [ ] 17. E2E 关键场景（创建/激活/合并/损坏/备份/导入导出）

  **What to do**:
  - 写 `e2e/create-config.spec.ts`：创建 + 编辑 + 保存 + 列表显示
  - 写 `e2e/apply-config.spec.ts`：
    - 正常路径：创建 → 应用 → 验证 opencode.jsonc 被修改 → 验证 backup 存在 → 验证 active.json 更新
    - 合并模式：fixture 含 model.headers，应用后 headers 保留
    - 损坏：fixture 是损坏 JSON，应用按钮 disabled 或弹错
  - 写 `e2e/backup-restore.spec.ts`：备份存在 → 恢复 → opencode 文件回到原状
  - 写 `e2e/import-export.spec.ts`：导出 → 删除 → 导入 → 内容一致
  - 写 `e2e/active-status.spec.ts`：4 种状态徽章
  - 写 `e2e/from-opencode.spec.ts`：真实 opencode.jsonc → 导入 → 字段填充正确

  **Must NOT do**:
  - 不让 E2E 跑超过 60s
  - 不让 E2E 写真实 `~/.config/opencode/`（必须隔离目录）
  - 不让 E2E 依赖网络

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 跨端到端测试，需要理解所有模块
  - **Skills**: [`playwright`, `test-driven-development`]
    - 理由：E2E 测试驱动

  **Parallelization**:
  - **Can Run In Parallel**: NO（必须等 T16 + T15）
  - **Parallel Group**: Wave 4
  - **Blocks**: T18
  - **Blocked By**: T15, T16, T10

  **References**:
  - Playwright fixtures: https://playwright.dev/docs/test-fixtures
  - tauri-driver usage: https://v2.tauri.app/develop/tests/webdriver/

  **Acceptance Criteria**:
  - [ ] `bun run e2e` 全过
  - [ ] 6 个 spec 文件覆盖 6 类关键场景
  - [ ] 每个 spec 有 happy + failure 路径

  **QA Scenarios**:
  ```
  Scenario: 全套 E2E
    Tool: Bash
    Steps:
      1. cd <project>
      2. bun run e2e
    Expected: 全部 PASS
    Evidence: .omo/evidence/task-17-e2e-full.txt
  ```

  **Commit**: `test(e2e): 关键场景 E2E`

---

- [ ] 18. Clippy + Lint + CI 配置 + README

  **What to do**:
  - 写 `.github/workflows/ci.yml`：
    - 触发：push + pull_request
    - jobs: `rust-test` (cargo test + clippy) / `frontend-lint` (lint + typecheck + test) / `e2e` (tauri build + playwright)
  - 写 `.github/workflows/release.yml`：tag 触发三平台打包
  - 写 `clippy.toml`：`# 禁止这些 pedantic lints`
  - 写 `rustfmt.toml`：标准配置
  - 写 `eslint.config.js` + `prettier.config.js`
  - 写 `README.md`：
    - 项目简介
    - 截图（待 Wave 3 完成后补）
    - 开发命令（dev / build / test / e2e）
    - 三平台打包命令
    - 已知限制（MVP 不含代码签名等）

  **Must NOT do**:
  - 不在 CI 加覆盖率门槛（只是报告）
  - 不让 lint 过不了也能 merge

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 标准 CI 配置
  - **Skills**: [`git-master`]
    - 理由：CI 与 git 工作流

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T16 平行）
  - **Parallel Group**: Wave 4
  - **Blocks**: T19
  - **Blocked By**: T1, T11

  **References**:
  - GitHub Actions: https://docs.github.com/en/actions
  - Tauri CI: https://v2.tauri.app/distribute/pipelines/github/

  **Acceptance Criteria**:
  - [ ] `bun run lint` 无错
  - [ ] `cargo clippy -- -D warnings` 无警告
  - [ ] CI 配置文件语法正确（用 `act` 验证本地）

  **QA Scenarios**:
  ```
  Scenario: Lint + Typecheck
    Tool: Bash
    Steps:
      1. cd <project>
      2. bun run lint
      3. bun run typecheck
      4. cd src-tauri && cargo clippy -- -D warnings
    Expected: 全部 0 errors / 0 warnings
    Evidence: .omo/evidence/task-18-lint.txt
  ```

  **Commit**: `chore(ci): GitHub Actions + Lint 配置`

---

- [ ] 19. 三平台打包脚本 + GitHub Actions

  **What to do**:
  - 写 `scripts/build-mac.sh`：`bun run tauri build --bundles app,dmg`
  - 写 `scripts/build-win.sh`：`bun run tauri build --bundles msi,nsis`（用 cross 或 Windows runner）
  - 写 `scripts/build-linux.sh`：`bun run tauri build --bundles deb,appimage`
  - 完善 `.github/workflows/release.yml`：tag 触发 → 三平台并行打包 → 上传 artifacts
  - 配置 `tauri.conf.json` 的 `bundle.publisher` / `bundle.category` / `bundle.shortDescription`
  - 写 `CHANGELOG.md` 初始版本
  - 写 `LICENSE` (MIT)

  **Must NOT do**:
  - 不在 MVP 阶段做代码签名
  - 不在 release.yml 加自动 publish 到 brew/scoop

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 标准打包脚本
  - **Skills**: []
    - 理由：模板化

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T18 平行）
  - **Parallel Group**: Wave 4 收尾
  - **Blocks**: F1-F4
  - **Blocked By**: T11, T18

  **References**:
  - Tauri 打包: https://v2.tauri.app/distribute/
  - Tauri GitHub Action: https://github.com/tauri-apps/tauri-action

  **Acceptance Criteria**:
  - [ ] macOS 打包成功生成 .app + .dmg
  - [ ] Linux 打包成功生成 .deb + .AppImage
  - [ ] Windows 打包配置正确（无 mac 机器只能配置，CI 验证）

  **QA Scenarios**:
  ```
  Scenario: macOS 打包
    Tool: Bash
    Steps:
      1. cd <project>
      2. ./scripts/build-mac.sh
    Expected: src-tauri/target/release/bundle/macos/*.app 和 dmg 存在
    Evidence: .omo/evidence/task-19-build-mac.txt

  Scenario: Linux 打包
    Tool: Bash
    Steps:
      1. cd <project>
      2. ./scripts/build-linux.sh (在 Linux 环境)
    Expected: .deb 和 .AppImage 存在
    Evidence: .omo/evidence/task-19-build-linux.txt
  ```

  **Commit**: `chore(release): 三平台打包脚本 + GitHub Actions`

---

## Final Verification Wave (MANDATORY — ALL implementation tasks done)

> 4 个 review agent 并行运行。**全部必须 APPROVE**。把合并结果给用户，**等用户明确同意**才能标记完成。
> **不要**自动通过验证。等用户明确 ok。
> 在得到用户 ok 之前**永远不要**勾选 F1-F4。被拒或用户反馈 → 修复 → 重跑 → 再展示 → 再等 ok。

- [ ] F1. **Plan Compliance Audit** — `oracle`
  逐项读 plan。对每个 Must Have：确认实现存在（读文件、跑命令、跑端点）。对每个 Must NOT Have：在代码库搜禁模式，命中就 file:line 拒。检查 evidence 文件存在。对照 plan 检 deliverables。
  输出：`Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  跑 plan "Success Criteria" 里的 build/lint/test 命令。审所有改过的文件：`#[allow(...)]`、空 catch、prod 里 debug 日志、注释掉的代码、未用的 import。查 AI slop：过度注释、过度抽象、泛型名（data/result/item/temp）。
  输出：`Build [PASS/FAIL] | Lint [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high` (+ `playwright` skill if UI)
  从干净状态开始。跑所有 task 的 QA scenario —— 跟精确步骤、capture evidence。测跨 task 集成（features 一起工作，不是孤岛）。测边界：空状态、非法输入、连续操作。存到 `e2e/screenshots/`。
  输出：`Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  对每个 task：读 "What to do"、读实际 diff（git log/diff）。1:1 验 —— spec 要求的都建了（没漏）、spec 外没建（无蔓延）。查 "Must NOT do" 合规。查跨 task 污染：Task N 改了 Task M 的文件。标记未计入的改动。
  输出：`Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- **Wave 1 完成时**：`feat(scaffold): 初始化 Tauri v2 + 基础模块 (T1-T5)`
- **Wave 2 完成时**：`feat(core): 合并 + 备份 + 命令层 (T6-T10)` — T6 单独提：`feat(merge): 深度合并 provider 块`
- **Wave 3 完成时**：`feat(ui): 列表 + 编辑 + 导入导出 (T11-T15)`
- **Wave 4 完成时**：`chore(quality): E2E + CI + 打包 (T16-T19)`
- **修复**：用 `fix(scope): 问题简述`

每次 commit 前：`cargo test && cargo clippy && bun run lint && bun run test`

---

## Success Criteria

### Verification Commands
```bash
# 后端
cd src-tauri && cargo test --all-features         # Expected: N passed; 0 failed
cd src-tauri && cargo clippy -- -D warnings      # Expected: no warnings
cd src-tauri && cargo build --release            # Expected: success

# 前端
bun run lint                                       # Expected: no errors
bun run typecheck                                  # Expected: no errors
bun run test                                       # Expected: all passed

# E2E
bun run e2e                                        # Expected: all passed

# 打包
bun run tauri build                                # Expected: 3 platforms success
```

### Final Checklist
- [ ] 所有 Must Have 实现
- [ ] 所有 Must NOT Have 不存在
- [ ] 合并 12 边界 100% 覆盖
- [ ] 三平台打包成功
- [ ] E2E 关键场景全过
- [ ] F1-F4 全部 APPROVE
- [ ] 用户明确同意
