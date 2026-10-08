import { it, expect, vi } from "vitest";
import {
  render,
  fireEvent,
  within,
  act,
  cleanup,
} from "@testing-library/react";
import TasksPage from "./TasksPage";
import { store } from "../lib/store";
import { newJob } from "../lib/jobs";
import type * as DesktopApi from "../lib/desktop";
import {
  defaultSettings,
  type Job,
  type Json,
  type Snapshot,
} from "../lib/types";
const remote = vi.hoisted(() => ({ call: vi.fn() }));
vi.mock("../lib/desktop", async (original) => ({
  ...(await original<typeof DesktopApi>()),
  workerCall: remote.call,
}));
it("任务乙请求完成不会解锁仍在执行的任务甲", async () => {
  const initial = store.state;
  const responses = new Map<string, (value: Json) => void>();
  remote.call.mockImplementation(
    (_method: string, body: { ids: string[] }) =>
      new Promise<Json>((resolve) => responses.set(body.ids[0], resolve)),
  );
  const jobs: Job[] = ["甲", "乙"].map((id) => ({
    ...newJob(defaultSettings),
    id,
    monitor_status: false,
    platform: "测试平台",
    platform_key: "custom",
    status_info: "已停止",
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
  }));
  const snapshot: Snapshot = {
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
  store.update({
    loaded: true,
    worker: { status: "connected", error: "" },
    snapshot,
    ui: { IsCardLayout: true, VisibleColumns: {} },
  });
  try {
    const view = render(<TasksPage />);
    const cards = view.container.querySelectorAll("article");
    const first = within(cards[0] as HTMLElement).getByRole("button", {
      name: "监控",
    }) as HTMLButtonElement;
    const second = within(cards[1] as HTMLElement).getByRole("button", {
      name: "监控",
    }) as HTMLButtonElement;
    fireEvent.click(first);
    fireEvent.click(second);
    expect(first.disabled).toBe(true);
    expect(second.disabled).toBe(true);
    await act(async () => {
      responses.get("乙")!({});
    });
    expect(second.disabled).toBe(false);
    expect(first.disabled).toBe(true);
    await act(async () => {
      responses.get("甲")!({});
    });
    expect(first.disabled).toBe(false);
  } finally {
    cleanup();
    store.update(initial);
    remote.call.mockReset();
  }
});
