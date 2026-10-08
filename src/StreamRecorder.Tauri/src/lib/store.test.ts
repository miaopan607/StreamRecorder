import { it, expect, vi, beforeEach, afterEach } from "vitest";
import { SettingsDraft } from "./store";
import { defaultSettings, type CoreSettings } from "./types";
beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  vi.clearAllTimers();
  vi.useRealTimers();
});
it("保存中继续编辑时，旧响应与快照都不会覆盖最新值", async () => {
  let persisted = { ...defaultSettings };
  let first: ((value: CoreSettings) => void) | undefined;
  let writes = 0;
  const draft = new SettingsDraft(
    defaultSettings,
    async (settings) => {
      writes++;
      if (writes === 1)
        return await new Promise<CoreSettings>((resolve) => {
          first = resolve;
        });
      persisted = { ...settings };
      return { ...persisted };
    },
    () => {},
  );
  draft.set("loop_time_seconds", "11");
  const saving = draft.flush();
  draft.set("loop_time_seconds", "22");
  draft.set("custom_filename_template", "最新模板🎬");
  draft.apply({ ...defaultSettings, loop_time_seconds: "11" });
  expect(draft.values.loop_time_seconds).toBe("22");
  expect(draft.values.custom_filename_template).toBe("最新模板🎬");
  first!({ ...defaultSettings, loop_time_seconds: "11" });
  await saving;
  expect(persisted.loop_time_seconds).toBe("22");
  expect(persisted.custom_filename_template).toBe("最新模板🎬");
  expect(draft.values.loop_time_seconds).toBe("22");
  expect(draft.dirty).toBe(false);
  await vi.advanceTimersByTimeAsync(2000);
  expect(writes).toBe(2);
});
it("500ms 内连续输入只保存最后值，回填不产生额外保存", async () => {
  let persisted = { ...defaultSettings };
  let writes = 0;
  const draft = new SettingsDraft(
    defaultSettings,
    async (settings) => {
      writes++;
      persisted = { ...settings };
      return { ...persisted };
    },
    () => {},
  );
  draft.set("custom_filename_template", "第一版");
  await vi.advanceTimersByTimeAsync(300);
  draft.set("custom_filename_template", "第二版");
  await vi.advanceTimersByTimeAsync(499);
  expect(persisted.custom_filename_template).toBe(
    defaultSettings.custom_filename_template,
  );
  await vi.advanceTimersByTimeAsync(1);
  expect(persisted.custom_filename_template).toBe("第二版");
  draft.apply({ ...persisted });
  await vi.advanceTimersByTimeAsync(2000);
  expect(writes).toBe(1);
});
it("失败保留脏字段，不定时重试；下一次修改可重新保存", async () => {
  let available = false;
  let writes = 0;
  const draft = new SettingsDraft(
    defaultSettings,
    async (settings) => {
      writes++;
      if (!available) throw new Error("核心已断开");
      return { ...settings };
    },
    () => {},
  );
  draft.set("live_save_path", "中文录制");
  await vi.advanceTimersByTimeAsync(500);
  expect(draft.dirty).toBe(true);
  expect(draft.error).toContain("核心已断开");
  draft.apply({ ...defaultSettings });
  expect(draft.values.live_save_path).toBe("中文录制");
  await vi.advanceTimersByTimeAsync(2000);
  expect(writes).toBe(1);
  available = true;
  draft.set("live_save_path", "恢复后录制");
  await vi.advanceTimersByTimeAsync(500);
  expect(draft.values.live_save_path).toBe("恢复后录制");
  expect(draft.error).toBe("");
  expect(draft.dirty).toBe(false);
});
