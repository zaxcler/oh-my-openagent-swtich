# oh-my-openagent-switch

跨平台桌面工具，简化 oh-my-openagent + opencode 的供应商切换流程。

## 功能

- 配置管理（CRUD）
- 一键应用（同时修改 oh-my-openagent.json + opencode.jsonc）
- 自动备份 + 恢复
- 导入/导出
- 激活状态提示

## 开发

```bash
bun install
bun run dev
```

## 构建

```bash
bun run tauri build
```

## 已知限制

- MVP 不含代码签名 / 公证 / 自动更新
- 自定义 opencode 路径暂不支持
