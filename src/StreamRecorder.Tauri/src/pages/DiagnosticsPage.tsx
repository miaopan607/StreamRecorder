import { useState } from "react";
import { store, useAppState } from "../lib/store";
import {
  pingCore,
  refreshDependencies,
  openDataFolder,
  writeText,
} from "../lib/desktop";
export default function DiagnosticsPage() {
  const state = useAppState();
  const [busy, setBusy] = useState("");
  const [notice, setNotice] = useState("");
  async function action(key: string, fn: () => Promise<unknown>) {
    setBusy(key);
    setNotice("");
    try {
      await fn();
    } catch (error) {
      store.update({ error: String(error) });
    } finally {
      setBusy("");
    }
  }
  const labels: Record<string, string> = { ffmpeg: "FFmpeg" };
  return (
    <section>
      <header className="page-header">
        <div>
          <h1>诊断</h1>
          <p>检查运行环境，查看核心服务日志。</p>
        </div>
      </header>
      <div className="diagnostic-layout">
        <aside className="diagnostic-info">
          <h2>核心服务</h2>
          <p>
            {state.core.status === "connected"
              ? "已就绪"
              : state.core.status === "starting"
                ? "正在启动核心服务"
                : "核心服务不可用"}
          </p>
          {state.core.error && (
            <p className="error-inline">{state.core.error}</p>
          )}
          {state.snapshot && (
            <p className="path-info">
              核心 {state.snapshot.app.version} · Rust
            </p>
          )}
          <ul className="dependency-list">
            {Object.entries(labels).map(([key, label]) => {
              const dependency = state.dependencies[key];
              return (
                <li key={key}>
                  <div className="dependency-title">
                    <strong>{label}</strong>
                    <span
                      className={`dependency-state ${dependency?.available ? "ready" : "missing"}`}
                    >
                      {dependency
                        ? dependency.available
                          ? "已就绪"
                          : "未找到"
                        : "正在检测"}
                    </span>
                  </div>
                  {dependency?.version && <small>{dependency.version}</small>}
                  {dependency?.path && <small>{dependency.path}</small>}
                  {dependency && !dependency.available && dependency.error && (
                    <small className="job-error">{dependency.error}</small>
                  )}
                </li>
              );
            })}
          </ul>
          <div className="inline-actions">
            <button
              disabled={state.core.status !== "connected" || busy === "ping"}
              onClick={() =>
                void action("ping", async () => {
                  const result = await pingCore();
                  setNotice(`核心服务响应正常：${result.utc}`);
                })
              }
            >
              测试核心服务
            </button>
            <button
              disabled={busy === "dependencies"}
              onClick={() =>
                void action("dependencies", async () =>
                  store.update({ dependencies: await refreshDependencies() }),
                )
              }
            >
              刷新依赖状态
            </button>
            <button onClick={() => void action("folder", openDataFolder)}>
              打开数据目录
            </button>
          </div>
          {notice && (
            <p className="path-info" role="status">
              {notice}
            </p>
          )}
          <p className="path-info muted">
            应用目录：{state.appRoot}
            <br />
            数据目录：{state.dataRoot}
          </p>
        </aside>
        <div className="logs-panel">
          <header className="logs-heading">
            <h2>运行日志</h2>
            <button
              disabled={!state.logs.length}
              onClick={() =>
                void action("copy", async () => {
                  await writeText(state.logs.join("\r\n"));
                  setNotice("日志已复制");
                })
              }
            >
              复制日志
            </button>
          </header>
          {state.logs.length ? (
            <ol className="logs-list">
              {state.logs.map((line, index) => (
                <li key={`${index}-${line}`}>{line}</li>
              ))}
            </ol>
          ) : (
            <p className="muted">暂无日志</p>
          )}
        </div>
      </div>
    </section>
  );
}
