import { beforeEach, expect, it, vi, type Mock } from "vitest";
import { AppStore } from "./store";
import { newJob } from "./jobs";
import { defaultSettings, type CoreState, type DesktopBootstrap, type Job, type Snapshot } from "./types";
import type * as DesktopApi from "./desktop";

const api = vi.hoisted(() => ({ listen: vi.fn(), bootstrap: vi.fn() }));
vi.mock("./desktop", async (original) => ({
  ...(await original<typeof DesktopApi>()),
  listen: api.listen,
  bootstrap: api.bootstrap,
}));
type Receiver = (event: { payload: unknown }) => void;
let listeners: Map<string, Receiver>;
let unlisten: Map<string, Mock>;
beforeEach(() => {
  listeners = new Map();
  unlisten = new Map();
  api.listen.mockReset();
  api.bootstrap.mockReset();
  api.listen.mockImplementation(async (name: string, receive: Receiver) => {
    listeners.set(name, receive);
    const stop = vi.fn(() => listeners.delete(name));
    unlisten.set(name, stop);
    return stop;
  });
});
function snapshot(revision: number, template: string, ids: string[]): Snapshot {
  const jobs: Job[] = ids.map((id) => ({
    ...newJob(defaultSettings), id, platform: "自定义直播流", platform_key: "custom",
    status_info: "监控中", display_title: id, title: id, error_message: "",
    created_at: "", updated_at: "", last_checked_at: "", live_title: "", record_url: "",
    latest_output_path: "", duration_text: "00:00:00", speed_text: "0 KB/s", recording_started_at: "",
  }));
  return {
    revision, jobs,
    app: { name: "StreamRecorder 核心", version: "1", ffmpeg_available: true, updated_at: "" },
    settings: { ...defaultSettings, custom_filename_template: template },
  };
}
function bootstrap(data: Snapshot, core: CoreState): DesktopBootstrap {
  return {
    snapshot: data, core_state: core, cookies: {}, accounts: {}, dependencies: {}, logs: [],
    ui_state: { IsCardLayout: true, VisibleColumns: {} }, autostart_enabled: false,
    app_root: "root", data_root: "root/runtime",
  };
}
it("bootstrap 与排队事件交错时只应用最新快照和核心状态", async () => {
  const app = new AppStore();
  app.update({ selected: new Set(["甲", "乙"]), active: "乙" });
  api.bootstrap.mockImplementation(async () => {
    listeners.get("core-event")!({ payload: { name: "snapshot_changed", body: snapshot(9, "旧模板", []) } });
    listeners.get("core-state")!({ payload: { status: "starting", error: "", revision: 9 } });
    listeners.get("core-event")!({ payload: { name: "snapshot_changed", body: snapshot(11, "最新模板", ["乙"]) } });
    return bootstrap(snapshot(10, "启动模板", ["甲", "乙"]), { status: "connected", error: "", revision: 10 });
  });
  await app.initialize();
  expect(app.state.snapshot?.revision).toBe(11);
  expect(app.state.settings.custom_filename_template).toBe("最新模板");
  expect([...app.state.selected]).toEqual(["乙"]);
  expect(app.state.active).toBe("乙");
  expect(app.state.core.status).toBe("connected");
  listeners.get("core-event")!({ payload: { name: "snapshot_changed", body: snapshot(10, "迟到模板", []) } });
  expect(app.state.settings.custom_filename_template).toBe("最新模板");
  expect(app.state.active).toBe("乙");
});
it("监听安装中途失败会撤销旧订阅，初始化可重试且不重复订阅", async () => {
  const app = new AppStore();
  const register = api.listen.getMockImplementation()!;
  api.listen.mockImplementation(async (name: string, receive: Receiver) => {
    if (name === "core-log") throw new Error("监听安装失败");
    return register(name, receive);
  });
  await app.initialize();
  expect(unlisten.get("core-event")!).toHaveBeenCalledOnce();
  expect(listeners.size).toBe(0);
  api.listen.mockImplementation(register);
  api.bootstrap.mockResolvedValue(bootstrap(snapshot(12, "恢复模板", ["甲"]), { status: "connected", error: "", revision: 12 }));
  await app.initialize();
  await app.initialize();
  expect([...listeners.keys()]).toEqual(["core-event", "core-log", "core-state"]);
  expect(app.state.core.status).toBe("connected");
  expect(app.state.error).toBe("");
  expect(app.state.settings.custom_filename_template).toBe("恢复模板");
  expect(api.bootstrap).toHaveBeenCalledOnce();
});
