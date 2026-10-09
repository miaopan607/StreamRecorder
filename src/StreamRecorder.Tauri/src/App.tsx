import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { store, useAppState } from "./lib/store";
import { listen, finishExit, cancelExit } from "./lib/desktop";
import TasksPage from "./pages/TasksPage";
import SettingsPage from "./pages/SettingsPage";
import DiagnosticsPage from "./pages/DiagnosticsPage";
import Modal from "./components/Modal";
export default function App() {
  const state = useAppState();
  const [page, setPage] = useState("tasks");
  const [exitFailed, setExitFailed] = useState(false);
  const discardNext = useRef(false);
  useEffect(() => {
    void store.initialize();
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<{ request_id: string }>(
      "desktop-exit-request",
      async (event) => {
        try {
          if (!discardNext.current) await store.draft.flush();
          discardNext.current = false;
          await finishExit(event.payload.request_id);
        } catch (error) {
          store.update({ error: `自动保存失败：${String(error)}` });
          await cancelExit(event.payload.request_id);
          setExitFailed(true);
        }
      },
    ).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
  useEffect(() => {
    const mode = state.settings.theme_mode;
    const system = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      document.documentElement.dataset.theme =
        mode === "system" ? (system.matches ? "dark" : "light") : mode;
      void getCurrentWindow()
        .setTheme(mode === "system" ? null : mode === "dark" ? "dark" : "light")
        .catch((error) => store.update({ error: String(error) }));
    };
    apply();
    system.addEventListener("change", apply);
    return () => system.removeEventListener("change", apply);
  }, [state.settings.theme_mode]);
  const jobs = state.snapshot?.jobs ?? [];
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark" />
          StreamRecorder
        </div>
        <p className="brand-subtitle">直播录制工具</p>
        <nav className="navigation" aria-label="主导航">
          {[
            ["tasks", "◉", "任务"],
            ["settings", "⚙", "设置"],
            ["diagnostics", "≡", "诊断"],
          ].map(([key, icon, label]) => (
            <button
              aria-current={page === key ? "page" : undefined}
              key={key}
              onClick={() => setPage(key)}
            >
              <span className="nav-icon" aria-hidden="true">
                {icon}
              </span>
              {label}
            </button>
          ))}
        </nav>
        <div className="sidebar-footer">
          <strong>录制由核心服务持续运行</strong>
          <br />
          关闭到托盘不会中断录制。
        </div>
      </aside>
      <main className="main-content">
        {state.error && (
          <div className="error-banner" role="alert">
            <span>{state.error}</span>
            <button
              className="icon-button"
              aria-label="关闭错误提示"
              onClick={() => store.update({ error: "" })}
            >
              ×
            </button>
          </div>
        )}
        <div hidden={page !== "tasks"}>
          <TasksPage />
        </div>
        <div hidden={page !== "settings"}>
          <SettingsPage />
        </div>
        <div hidden={page !== "diagnostics"}>
          <DiagnosticsPage />
        </div>
      </main>
      <footer className="status-bar">
        <span className="status-connection">
          <i className={`connection-dot ${state.core.status}`} />
          {state.core.status === "connected"
            ? "已就绪"
            : state.core.status === "starting"
              ? "正在启动核心服务"
              : "核心服务不可用"}
        </span>
        <span>{jobs.length} 个任务</span>
        <span>{jobs.filter((job) => job.monitor_status).length} 个监控中</span>
        <span className="status-right">
          {state.snapshot
            ? `核心 ${state.snapshot.app.version} · Rust`
            : "原生核心服务"}
        </span>
      </footer>
      {exitFailed && (
        <Modal title="设置尚未保存" onClose={() => setExitFailed(false)}>
          <div className="modal-body">
            <p>自动保存失败。可以继续编辑，或放弃未保存的设置并退出。</p>
          </div>
          <footer className="modal-footer">
            <button onClick={() => setExitFailed(false)}>继续使用</button>
            <button
              className="danger"
              onClick={() => {
                discardNext.current = true;
                setExitFailed(false);
                void invoke("request_exit");
              }}
            >
              放弃更改并退出
            </button>
          </footer>
        </Modal>
      )}
    </div>
  );
}
