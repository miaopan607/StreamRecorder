import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type { Json, CoreSettings, JobDraft, DesktopBootstrap, UiState } from "./types";
export { listen, writeText };
export const pingCore = () => invoke<{ utc: string }>("health_ping");
export const updateSettings = (settings: CoreSettings) =>
  invoke<{ settings: CoreSettings }>("settings_update", { settings });
export const getCookies = () =>
  invoke<{ cookies: Record<string, string> }>("cookies_get");
export const updateCookies = (cookies: Record<string, string>) =>
  invoke<{ cookies: Record<string, string> }>("cookies_update", { cookies });
export const getAccounts = () =>
  invoke<{ accounts: Record<string, Json> }>("accounts_get");
export const updateAccounts = (accounts: Record<string, Json>) =>
  invoke<{ accounts: Record<string, Json> }>("accounts_update", { accounts });
export const upsertJobs = (jobs: JobDraft[]) =>
  invoke<{ changed_ids: string[] }>("jobs_upsert", { jobs });
export const deleteJobs = (ids: string[]) =>
  invoke<{ deleted: number }>("jobs_delete", { ids });
export const startMonitoring = (ids: string[]) =>
  invoke<{ changed_ids: string[]; monitoring: true }>("jobs_start_monitoring", { ids });
export const stopMonitoring = (ids: string[]) =>
  invoke<{ changed_ids: string[]; monitoring: false }>("jobs_stop_monitoring", { ids });
export const recheckJobs = (ids: string[]) =>
  invoke<{ queued_ids: string[] }>("jobs_recheck", { ids });
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
