import { useState } from "react";
import { store, useAppState } from "../lib/store";
import {
  startMonitoring,
  stopMonitoring,
  recheckJobs,
  deleteJobs,
  saveUiState,
  openJobFolder,
  openLatestFile,
  writeText,
} from "../lib/desktop";
import { qualities, selectedTargets, columns, detailText } from "../lib/jobs";
import type { Job } from "../lib/types";
import Modal from "../components/Modal";
import JobEditor from "../components/JobEditor";
export default function TasksPage() {
  const state = useAppState();
  const jobs = state.snapshot?.jobs ?? [];
  const [editor, setEditor] = useState<Job | null | undefined>();
  const [details, setDetails] = useState<Job | null>(null);
  const [columnPicker, setColumnPicker] = useState<Record<
    string,
    boolean
  > | null>(null);
  const [deleting, setDeleting] = useState<Job[] | null>(null);
  const [busy, setBusy] = useState<Set<string>>(() => new Set());
  const connected = state.core.status === "connected";
  const targets = selectedTargets(jobs, state.selected, state.active, true);
  const deleteTargets = selectedTargets(
    jobs,
    state.selected,
    state.active,
    false,
  );
  const allTarget = !state.selected.size && !state.active;
  async function action(key: string, fn: () => Promise<unknown>) {
    setBusy((previous) => new Set([...previous, key]));
    try {
      await fn();
    } catch (error) {
      store.update({ error: String(error) });
    } finally {
      setBusy((previous) => {
        const next = new Set(previous);
        next.delete(key);
        return next;
      });
    }
  }
  async function command(run: (ids: string[]) => Promise<unknown>, targets: Job[], key = "batch") {
    if (targets.length)
      await action(key, () => run(targets.map((job) => job.id)));
  }
  async function layout() {
    const ui = { ...state.ui, IsCardLayout: !state.ui.IsCardLayout };
    await action("layout", async () => {
      await saveUiState(ui);
      store.update({ ui });
    });
  }
  function selectAll() {
    const clear = jobs.every((job) => state.selected.has(job.id));
    store.update({
      selected: clear ? new Set() : new Set(jobs.map((job) => job.id)),
      active: null,
    });
  }
  const visible = (key: string) => state.ui.VisibleColumns[key] !== false;
  const checkbox = (job: Job) => (
    <input
      type="checkbox"
      aria-label={`选择 ${job.display_title}`}
      checked={state.selected.has(job.id)}
      onClick={(event) => event.stopPropagation()}
      onChange={(event) => store.select(job.id, event.target.checked)}
    />
  );
  const status = (job: Job) => (
    <span
      className={`status ${job.status_info === "录制中" ? "recording" : job.status_info === "错误" ? "failed" : job.monitor_status ? "monitoring" : "stopped"}`}
    >
      {job.status_info}
    </span>
  );
  function actions(job: Job) {
    const disabled = !connected || busy.has(job.id) || busy.has("batch");
    return (
      <div className="job-actions" onClick={(event) => event.stopPropagation()}>
        <button disabled={disabled} onClick={() => setEditor(job)}>
          编辑
        </button>
        <button onClick={() => setDetails(job)}>详情</button>
        <button
          disabled={disabled}
          onClick={() => void command(recheckJobs, [job], job.id)}
        >
          重检
        </button>
        <button
          disabled={disabled}
          onClick={() =>
            void command(
              job.monitor_status ? stopMonitoring : startMonitoring,
              [job],
              job.id,
            )
          }
        >
          {job.monitor_status ? "停止" : "监控"}
        </button>
        <button
          disabled={!job.recording_dir && !job.latest_output_path && !connected}
          onClick={() => void action(job.id, () => openJobFolder(job.id))}
        >
          目录
        </button>
        <button
          disabled={!job.latest_output_path}
          onClick={() => void action(job.id, () => openLatestFile(job.id))}
        >
          播放
        </button>
      </div>
    );
  }
  function cell(job: Job, key: string) {
    switch (key) {
      case "PlatformColumn":
        return job.platform;
      case "UrlColumn":
        return (
          <span className="url" title={job.url}>
            {job.url}
          </span>
        );
      case "StatusColumn":
        return status(job);
      case "QualityColumn":
        return qualities[job.quality] ?? job.quality;
      case "FormatColumn":
        return job.record_format;
      case "DurationColumn":
        return job.duration_text;
      case "SpeedColumn":
        return job.speed_text;
      case "UpdatedAtColumn":
        return job.updated_at
          ? new Date(job.updated_at).toLocaleString("zh-CN")
          : "";
      default:
        return "";
    }
  }
  return (
    <section className="tasks-page">
      <header className="page-header">
        <div>
          <h1>任务</h1>
          <p>监控直播间，开播后自动录制。</p>
        </div>
        <button
          className="primary"
          disabled={!connected}
          onClick={() => setEditor(null)}
        >
          ＋ 添加任务
        </button>
      </header>
      <div className="task-toolbar">
        <div className="inline-actions">
          <button onClick={() => void layout()} disabled={busy.has("layout")}>
            {state.ui.IsCardLayout ? "列表布局" : "卡片布局"}
          </button>
          <button
            onClick={() => setColumnPicker({ ...state.ui.VisibleColumns })}
          >
            显示列
          </button>
          <span className="toolbar-divider" />
          <button onClick={selectAll} disabled={!jobs.length}>
            {jobs.length && jobs.every((job) => state.selected.has(job.id))
              ? "取消全选"
              : "全选"}
          </button>
          {(state.selected.size > 0 || state.active) && (
            <button
              onClick={() =>
                store.update({ selected: new Set(), active: null })
              }
            >
              取消选择
            </button>
          )}
        </div>
        <span className="selection-label">
          {allTarget ? "全部任务" : `已选 ${targets.length} 项`}
        </span>
        <div className="inline-actions">
          <button
            disabled={!connected || !jobs.length || busy.has("batch")}
            onClick={() => void command(startMonitoring, targets)}
          >
            开始监控
          </button>
          <button
            disabled={!connected || !jobs.length || busy.has("batch")}
            onClick={() => void command(stopMonitoring, targets)}
          >
            停止监控
          </button>
          <button
            disabled={!connected || !jobs.length || busy.has("batch")}
            onClick={() => void command(recheckJobs, targets)}
          >
            立即重检
          </button>
          <button
            className="danger-text"
            disabled={!connected || !deleteTargets.length || busy.has("batch")}
            onClick={() => setDeleting(deleteTargets)}
          >
            删除
          </button>
        </div>
      </div>
      {!jobs.length ? (
        <div className="empty-state">
          <span className="empty-mark">◉</span>
          <h2>{connected ? "还没有录制任务" : "核心服务未连接"}</h2>
          <p>
            {connected
              ? "添加一个直播间，录制会在开播后自动开始。"
              : "请到诊断页面检查核心服务运行状态。"}
          </p>
          {connected && (
            <button className="primary" onClick={() => setEditor(null)}>
              添加任务
            </button>
          )}
        </div>
      ) : state.ui.IsCardLayout ? (
        <div className="job-cards" key="cards">
          {jobs.map((job) => (
            <article
              className={`job-card ${state.active === job.id ? "selected" : ""}`}
              key={job.id}
              onClick={() => store.update({ active: job.id })}
            >
              <header>
                {checkbox(job)}
                <div>
                  <h2 title={job.display_title}>{job.display_title}</h2>
                  <small>{job.platform}</small>
                </div>
                {status(job)}
              </header>
              <p className="job-url" title={job.url}>
                {job.url}
              </p>
              <div className="job-meta">
                <span>
                  {qualities[job.quality] ?? job.quality} · {job.record_format}
                </span>
                <span className="mono">{job.duration_text}</span>
                <span className="mono">{job.speed_text}</span>
              </div>
              {job.error_message && (
                <p className="job-error" title={job.error_message}>
                  {job.error_message}
                </p>
              )}
              {actions(job)}
            </article>
          ))}
        </div>
      ) : (
        <div className="table-container" key="table">
          <table>
            <thead>
              <tr>
                <th className="selection-cell">选择</th>
                <th>标题</th>
                {columns
                  .filter(([key]) => visible(key))
                  .map(([key, label]) => (
                    <th key={key}>{label}</th>
                  ))}
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              {jobs.map((job) => (
                <tr
                  key={job.id}
                  className={state.active === job.id ? "selected" : ""}
                  onClick={() => store.update({ active: job.id })}
                >
                  <td>{checkbox(job)}</td>
                  <td>
                    <strong title={job.display_title}>
                      {job.display_title}
                    </strong>
                    {job.error_message && (
                      <small className="job-error">{job.error_message}</small>
                    )}
                  </td>
                  {columns
                    .filter(([key]) => visible(key))
                    .map(([key]) => (
                      <td key={key}>{cell(job, key)}</td>
                    ))}
                  <td>{actions(job)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {editor !== undefined && (
        <JobEditor job={editor} onClose={() => setEditor(undefined)} />
      )}
      {details && (
        <Modal title="任务详细信息" onClose={() => setDetails(null)}>
          <div className="modal-body">
            <pre className="detail-text">
              {detailText(jobs.find((job) => job.id === details.id) ?? details)}
            </pre>
          </div>
          <footer className="modal-footer">
            <button
              onClick={() =>
                void action("copy", () =>
                  writeText(
                    detailText(
                      jobs.find((job) => job.id === details.id) ?? details,
                    ),
                  ),
                )
              }
            >
              复制全部
            </button>
            <button className="primary" onClick={() => setDetails(null)}>
              关闭
            </button>
          </footer>
        </Modal>
      )}
      {columnPicker && (
        <Modal title="显示列" onClose={() => setColumnPicker(null)}>
          <div className="modal-body column-options">
            <label className="check-field">
              <input type="checkbox" checked disabled />
              标题（固定）
            </label>
            {columns.map(([key, label]) => (
              <label className="check-field" key={key}>
                <input
                  type="checkbox"
                  checked={columnPicker[key] !== false}
                  onChange={(event) =>
                    setColumnPicker({
                      ...columnPicker,
                      [key]: event.target.checked,
                    })
                  }
                />
                {label}
              </label>
            ))}
            <label className="check-field">
              <input type="checkbox" checked disabled />
              操作（固定）
            </label>
          </div>
          <footer className="modal-footer">
            <button onClick={() => setColumnPicker(null)}>取消</button>
            <button
              className="primary"
              onClick={() =>
                void action("columns", async () => {
                  const ui = { ...state.ui, VisibleColumns: columnPicker };
                  await saveUiState(ui);
                  store.update({ ui });
                  setColumnPicker(null);
                })
              }
            >
              确定
            </button>
          </footer>
        </Modal>
      )}
      {deleting && (
        <Modal title="删除任务" onClose={() => setDeleting(null)}>
          <div className="modal-body">
            <p>确定删除选中的 {deleting.length} 个任务吗？</p>
            <p className="muted">
              正在录制的任务会停止。已有录制文件不会删除。
            </p>
          </div>
          <footer className="modal-footer">
            <button
              disabled={busy.has("batch")}
              onClick={() => setDeleting(null)}
            >
              取消
            </button>
            <button
              className="danger"
              disabled={busy.has("batch")}
              onClick={() =>
                void action("batch", async () => {
                  await deleteJobs(deleting.map((job) => job.id));
                  setDeleting(null);
                })
              }
            >
              确定删除
            </button>
          </footer>
        </Modal>
      )}
    </section>
  );
}
