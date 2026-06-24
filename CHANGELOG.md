# Changelog

All notable changes to this project will be documented in this file.

## [0.1.1] - 2026-06-24

### Added

- Model 多模态 (`modalities`) 字段填写:opencode 支持的 text / image / audio / video / pdf 五种 modality 可在 ModelRow 折叠面板中勾选,声明 model 的 input / output 能力
- 同步支持导入现有 opencode.jsonc 的 `modalities` 字段,避免导入即丢失

### Changed

- 后端 `ModelEntry` 增 `modalities: Option<Modalities>` 字段,`skip_serializing_if` 实现向后兼容(未填或全空时 JSON 序列化结果与 0.1.0 完全一致)
- ModelRow 顶部行加 🎨 多模态 折叠按钮,有值时自动展开

## [0.1.0] - 2026-06-20

### Added

- 配置 CRUD（创建/读取/更新/删除）
- 一键应用（深度合并 provider.omos + 整体替换 oh-my-openagent.json）
- 自动备份 + 恢复
- 导入/导出
- 激活状态提示
- 12 边界合并 TDD 测试
- E2E 测试框架
- 三平台 CI + 打包脚本
