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
  startMonitoring: remote.call,
}));
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
  revision: 1,
  app: {
    name: "核心",
    version: "1",
    ffmpeg_available: true,
    updated_at: "",
  },
  settings: { ...defaultSettings },
  jobs,
};
it("任务乙请求完成不会解锁仍在执行的任务甲", async () => {
  const initial = store.state;
  const responses = new Map<string, (value: Json) => void>();
  remote.call.mockImplementation(
    (ids: string[]) =>
      new Promise<Json>((resolve) => responses.set(ids[0], resolve)),
  );
  store.update({
    loaded: true,
    core: { status: "connected", error: "", revision: 1 },
    snapshot,
    selected: new Set(),
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

it.each([
  ["表格", false],
  ["卡片", true],
] as const)("%s条目的各区域与复选框共用选择状态", async (_layout, isCardLayout) => {
  const initial = store.state;
  store.update({
    loaded: true,
    core: { status: "connected", error: "", revision: 1 },
    snapshot,
    selected: new Set(),
    ui: { IsCardLayout: isCardLayout, VisibleColumns: {} },
  });
  try {
    const view = render(<TasksPage />);
    const first = view.getByRole("checkbox", {
      name: "选择 甲",
    }) as HTMLInputElement;
    const second = view.getByRole("checkbox", {
      name: "选择 乙",
    }) as HTMLInputElement;
    const firstEntry = first.closest(
      isCardLayout ? "article" : "tr",
    ) as HTMLElement;
    const secondEntry = second.closest(
      isCardLayout ? "article" : "tr",
    ) as HTMLElement;
    const remove = view.getByRole("button", {
      name: "删除",
    }) as HTMLButtonElement;

    fireEvent.click(within(firstEntry).getByText("甲"));
    expect(first.checked).toBe(true);
    expect(remove.disabled).toBe(false);

    fireEvent.click(within(firstEntry).getByText("已停止"));
    expect(first.checked).toBe(false);
    expect(remove.disabled).toBe(true);

    fireEvent.click(firstEntry);
    fireEvent.click(secondEntry.querySelector(".job-actions")!);
    expect(first.checked).toBe(true);
    expect(second.checked).toBe(true);
    expect(view.getByText("已选 2 项")).toBeDefined();

    fireEvent.click(first);
    expect(first.checked).toBe(false);
    expect(second.checked).toBe(true);
    fireEvent.click(second);
    expect(second.checked).toBe(false);
    expect(remove.disabled).toBe(true);

    remote.call.mockResolvedValue({});
    await act(async () => {
      fireEvent.click(within(firstEntry).getByRole("button", { name: "监控" }));
    });
    expect(first.checked).toBe(true);
    await act(async () => {
      fireEvent.click(within(firstEntry).getByRole("button", { name: "监控" }));
    });
    expect(first.checked).toBe(false);
  } finally {
    cleanup();
    store.update(initial);
    remote.call.mockReset();
  }
});
