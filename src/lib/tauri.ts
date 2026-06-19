/**
 * Tauri invoke 包装层。
 *
 * - 集中处理错误日志
 * - 保持与 Rust 端 `#[tauri::command]` 签名一一对应
 * - 业务层调用 `tauriInvoke<T>(name, args)` 即可享受一致的错误语义
 *
 * 严禁在前端业务中直接调用 `@tauri-apps/api/core`，所有跨进程调用必须经此层。
 */
import { invoke } from '@tauri-apps/api/core';

/**
 * Tauri command 调用统一包装。
 *
 * @param command Rust 端 `#[tauri::command]` 函数名
 * @param args 命令参数（snake_case 字段，与 Rust 结构体一致）
 * @returns Rust 端返回的 `Ok(T)` 中的 `T`
 * @throws Rust 端 `Err(AppError)` 序列化的字符串错误信息
 */
export async function tauriInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    // 业务层可继续 `throw`；这里只补充上下文日志，便于排查。
    console.error(`[tauriInvoke] command="${command}" failed:`, error);
    throw error;
  }
}
