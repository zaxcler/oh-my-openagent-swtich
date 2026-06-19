//! 存储模块
//!
//! 路径解析 (`paths`)、目录初始化 (`init`)、配置 CRUD (`configs`)、
//! 激活状态 (`active` + `detect`)、导入导出 (`import` + `export`)、
//! 备份管理 (`backup`) 和原子写入 (`atomic`)

pub mod active;
pub mod atomic;
pub mod backup;
pub mod configs;
pub mod detect;
pub mod export;
pub mod import;
pub mod init;
pub mod paths;
