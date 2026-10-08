import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { parseBatch, newJob, selectedTargets } from "./jobs";
import { AppStore } from "./store";
import { defaultSettings, type Job, type Snapshot } from "./types";
function job(id: string): Job {
  return {
    ...newJob(defaultSettings),
    id,
    platform: "自定义直播流",
    platform_key: "custom",
    status_info: "监控中",
    display_title: id,
    title: id,
    error_message: "",
    created_at: "",
    updated_at: "",
    last_checked_at: "",
    live_title: "",
    record_url: "",
    latest_output_path: "",
    duration_text: "00:00:00",
    speed_text: "0 KB/s",
    recording_started_at: "",
  };
}
beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  vi.clearAllTimers();
  vi.useRealTimers();
});
function snapshot(jobs: Job[]): Snapshot {
  return {
    app: {
      name: "核心",
      version: "1",
      python_version: "3",
      ffmpeg_available: true,
      node_available: true,
      updated_at: "",
    },
    settings: { ...defaultSettings },
    jobs,
  };
}
describe("批量导入", () => {
  it("处理中英文逗号、编号、默认画质和重复链接", () => {
    const rows = parseBatch(
      "2，https://a.test/live，主播甲\r\nHTTPS://A.TEST/LIVE,重复\nhttps://b.test/live,主播乙\n4,https://c.test/live\n无效说明\n9,https://d.test/live,主播丁",
      defaultSettings,
    );
    expect(
      rows.map((row) => [row.url, row.quality, row.streamer_name]),
    ).toEqual([
      ["https://a.test/live", "HD", "主播甲"],
      ["https://b.test/live", "OD", "主播乙"],
      ["https://c.test/live", "LD", "直播间"],
      ["https://d.test/live", "OD", "主播丁"],
    ]);
  });
  it("没有有效行时不给出可提交任务", () =>
    expect(parseBatch("说明文字\n,", defaultSettings)).toEqual([]));
});
describe("批量目标与快照", () => {
  it("复选集合优先于当前行，无选择监控全部但删除不扩大范围", () => {
    const jobs = [job("a"), job("b"), job("c")];
    expect(
      selectedTargets(jobs, new Set(["b"]), "a", true).map((job) => job.id),
    ).toEqual(["b"]);
    expect(
      selectedTargets(jobs, new Set(), "c", true).map((job) => job.id),
    ).toEqual(["c"]);
    expect(
      selectedTargets(jobs, new Set(), null, true).map((job) => job.id),
    ).toEqual(["a", "b", "c"]);
    expect(selectedTargets(jobs, new Set(), null, false)).toEqual([]);
  });
  it("刷新保持存在的选择，删除清理失效选择，不丢配置草稿", () => {
    const app = new AppStore();
    app.applySnapshot(snapshot([job("a"), job("b")]));
    app.update({ active: "b", selected: new Set(["a", "b"]) });
    app.draft.set("custom_filename_template", "用户输入");
    app.applySnapshot(snapshot([job("b")]));
    expect([...app.state.selected]).toEqual(["b"]);
    expect(app.state.active).toBe("b");
    expect(app.state.settings.custom_filename_template).toBe("用户输入");
    app.applySnapshot(snapshot([]));
    expect(app.state.active).toBeNull();
    expect([...app.state.selected]).toEqual([]);
  });
});
