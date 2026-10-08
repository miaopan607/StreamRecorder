import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type { Json, WorkerMethod, DesktopBootstrap, UiState } from "./types";
export { listen, writeText };
export const workerCall = <T = Json>(
  method: WorkerMethod,
  body: unknown = {},
) => invoke<T>("worker_call", { method, body });
export const bootstrap = () => invoke<DesktopBootstrap>("desktop_bootstrap");
export const saveUiState = (state: UiState) =>
  invoke<void>("ui_state_set", { state });
export const setAutostart = (enabled: boolean) =>
  invoke<void>("set_autostart", { enabled });
export const openJobFolder = (id: string) =>
  invoke<void>("open_job_folder", { id });
export const openLatestFile = (id: string) =>
  invoke<void>("open_latest_file", { id });
export const openDataFolder = () => invoke<void>("open_data_folder");
export const refreshDependencies = () =>
  invoke<DesktopBootstrap["dependencies"]>("refresh_dependencies");
export const finishExit = (requestId: string) =>
  invoke<void>("finish_exit", { requestId });
export const cancelExit = (requestId: string) =>
  invoke<void>("cancel_exit", { requestId });
export const selectFolder = () =>
  open({ directory: true, multiple: false, title: "选择保存目录" });
