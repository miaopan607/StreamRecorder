import { useState } from "react";
import Modal from "./Modal";
import Field from "./Field";
import { selectFolder, workerCall } from "../lib/desktop";
import { formats, qualities, newJob, jobDraft, parseBatch } from "../lib/jobs";
import { useAppState } from "../lib/store";
import type { Job, JobDraft } from "../lib/types";
export default function JobEditor({
  job,
  onClose,
}: {
  job: Job | null;
  onClose: () => void;
}) {
  const state = useAppState();
  const [draft, setDraft] = useState<JobDraft>(() =>
    job ? jobDraft(job) : newJob(state.settings),
  );
  const [batch, setBatch] = useState(false);
  const [text, setText] = useState("");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const change = (key: keyof JobDraft, value: string | boolean) =>
    setDraft((previous) => ({ ...previous, [key]: value }));
  async function save(event: React.SyntheticEvent) {
    event.preventDefault();
    setError("");
    let jobs: JobDraft[];
    if (batch) {
      jobs = parseBatch(text, state.settings);
      if (!jobs.length) {
        setError("没有找到有效的批量任务。每行填写：画质编号,链接,主播名");
        return;
      }
    } else {
      if (!draft.url.trim()) {
        setError("直播间链接不能为空");
        return;
      }
      jobs = [
        {
          ...draft,
          url: draft.url.trim(),
          streamer_name: draft.streamer_name.trim() || "直播间",
          segment_time:
            draft.segment_time.trim() || state.settings.video_segment_time,
          monitor_hours: draft.monitor_hours.trim() || "5",
          recording_dir: draft.recording_dir.trim(),
        },
      ];
    }
    setSaving(true);
    try {
      await workerCall("jobs.upsert", { jobs });
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }
  return (
    <Modal
      title={job ? "编辑任务" : "添加任务"}
      drawer
      onClose={() => {
        if (!saving) onClose();
      }}
    >
      <form onSubmit={save}>
        <div className="drawer-body">
          {!job && (
            <div className="segmented" role="tablist">
              <button
                type="button"
                role="tab"
                aria-selected={!batch}
                onClick={() => setBatch(false)}
              >
                单条
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={batch}
                onClick={() => setBatch(true)}
              >
                批量导入
              </button>
            </div>
          )}
          {batch ? (
            <>
              <label className="field">
                <span>每行一个直播间</span>
                <textarea
                  className="batch-input"
                  aria-label="每行一个直播间"
                  value={text}
                  onChange={(event) => setText(event.target.value)}
                  placeholder={
                    "0,https://直播间链接,主播名\nhttps://直播间链接,主播名"
                  }
                />
                <small>
                  0 原画 · 1 超清 · 2 高清 · 3 标清 · 4
                  流畅。支持中英文逗号，重复链接仅导入一次。
                </small>
              </label>
            </>
          ) : (
            <>
              <Field
                label="直播间链接"
                value={draft.url}
                onChange={(value) => change("url", value)}
              />
              <Field
                label="主播名"
                value={draft.streamer_name}
                onChange={(value) => change("streamer_name", value)}
              />
              <div className="form-grid">
                <Field
                  label="画质"
                  value={draft.quality}
                  options={qualities}
                  onChange={(value) => change("quality", value)}
                />
                <Field
                  label="录制格式"
                  value={draft.record_format}
                  options={Object.fromEntries(
                    formats.map((format) => [format, format]),
                  )}
                  onChange={(value) => change("record_format", value)}
                />
              </div>
              <Field
                label="保存目录"
                value={draft.recording_dir}
                onChange={(value) => change("recording_dir", value)}
                hint="留空使用全局目录；相对路径按程序所在目录解析。"
              />
              <div className="inline-actions">
                <button
                  type="button"
                  onClick={() => change("recording_dir", "")}
                >
                  默认目录
                </button>
                <button
                  type="button"
                  onClick={async () => {
                    try {
                      const selected = await selectFolder();
                      if (typeof selected === "string")
                        change("recording_dir", selected);
                    } catch (e) {
                      setError(String(e));
                    }
                  }}
                >
                  选择目录…
                </button>
              </div>
              <div className="form-section">
                <h3>监控与录制</h3>
                <Field
                  label="开启监控"
                  value={draft.monitor_status}
                  onChange={(value) => change("monitor_status", value)}
                />
                <Field
                  label="分段录制"
                  value={draft.segment_record}
                  onChange={(value) => change("segment_record", value)}
                />
                <Field
                  label="分段时长（秒）"
                  value={draft.segment_time}
                  disabled={!draft.segment_record}
                  onChange={(value) => change("segment_time", value)}
                />
                <Field
                  label="定时录制"
                  value={draft.scheduled_recording}
                  onChange={(value) => change("scheduled_recording", value)}
                />
                <Field
                  label="计划开始时间"
                  value={draft.scheduled_start_time}
                  disabled={!draft.scheduled_recording}
                  hint="格式：YYYY-MM-DD HH:mm:ss"
                  onChange={(value) => change("scheduled_start_time", value)}
                />
                <Field
                  label="监控时长（小时）"
                  value={draft.monitor_hours}
                  onChange={(value) => change("monitor_hours", value)}
                />
              </div>
              <div className="form-section">
                <h3>通知与下载</h3>
                <Field
                  label="启用消息推送"
                  value={draft.enabled_message_push}
                  onChange={(value) => change("enabled_message_push", value)}
                />
                <Field
                  label="仅通知，不录制"
                  value={draft.only_notify_no_record}
                  onChange={(value) => change("only_notify_no_record", value)}
                />
                <Field
                  label="FLV 直连下载"
                  value={draft.flv_use_direct_download}
                  onChange={(value) => change("flv_use_direct_download", value)}
                />
              </div>
            </>
          )}
          {error && (
            <div className="error-inline" role="alert">
              {error}
            </div>
          )}
        </div>
        <footer className="modal-footer">
          <button type="button" disabled={saving} onClick={onClose}>
            取消
          </button>
          <button
            type="submit"
            className="primary"
            disabled={saving || state.worker.status !== "connected"}
          >
            {saving ? "正在提交…" : "确定"}
          </button>
        </footer>
      </form>
    </Modal>
  );
}
