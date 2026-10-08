import { useSyncExternalStore } from "react";
import { bootstrap, listen, workerCall } from "./desktop";
import {
  defaultSettings,
  type CoreSettings,
  type DesktopBootstrap,
  type Snapshot,
  type UiState,
  type WorkerEvent,
  type WorkerState,
} from "./types";

// 配置草稿独立于快照，回填只覆盖没有本地修改的字段。
export class SettingsDraft {
  values: CoreSettings;
  error = "";
  saving = false;
  private revisions = new Map<string, number>();
  private revision = 0;
  private timer: number | undefined;
  private flight: Promise<void> | undefined;
  constructor(
    initial: CoreSettings,
    private write: (settings: CoreSettings) => Promise<CoreSettings>,
    private changed: () => void,
  ) {
    this.values = { ...initial };
  }
  get dirty() {
    return this.revisions.size > 0;
  }
  apply(settings: CoreSettings) {
    const values = { ...this.values };
    for (const [key, value] of Object.entries(settings)) {
      if (!this.revisions.has(key)) values[key] = value;
    }
    this.values = values;
    this.changed();
  }
  set(key: string, value: CoreSettings[string]) {
    this.values = { ...this.values, [key]: value };
    this.revisions.set(key, ++this.revision);
    this.error = "";
    this.changed();
    clearTimeout(this.timer);
    this.timer = window.setTimeout(() => {
      void this.flush().catch(() => {});
    }, 500);
  }
  async flush() {
    clearTimeout(this.timer);
    while (this.dirty) {
      if (this.flight) {
        await this.flight;
        continue;
      }
      this.saving = true;
      this.changed();
      const sent = new Map(this.revisions);
      const values = { ...this.values };
      this.flight = (async () => {
        try {
          const result = await this.write(values);
          for (const [key, version] of sent) {
            if (this.revisions.get(key) === version) this.revisions.delete(key);
          }
          this.error = "";
          this.apply(result);
        } catch (error) {
          this.error = String(error);
          throw error;
        } finally {
          this.saving = false;
          this.flight = undefined;
          this.changed();
        }
      })();
      await this.flight;
    }
  }
}
export interface AppState {
  loaded: boolean;
  snapshot: Snapshot | null;
  settings: CoreSettings;
  worker: WorkerState;
  cookies: DesktopBootstrap["cookies"];
  accounts: DesktopBootstrap["accounts"];
  dependencies: DesktopBootstrap["dependencies"];
  ui: UiState;
  autostart: boolean;
  logs: string[];
  selected: Set<string>;
  active: string | null;
  error: string;
  saveError: string;
  saving: boolean;
  dirty: boolean;
  appRoot: string;
  dataRoot: string;
}
export class AppStore {
  state: AppState = {
    loaded: false,
    snapshot: null,
    settings: { ...defaultSettings },
    worker: { status: "starting", error: "" },
    cookies: {},
    accounts: {},
    dependencies: {},
    ui: { IsCardLayout: true, VisibleColumns: {} },
    autostart: false,
    logs: [],
    selected: new Set(),
    active: null,
    error: "",
    saveError: "",
    saving: false,
    dirty: false,
    appRoot: "",
    dataRoot: "",
  };
  private subscribers = new Set<() => void>();
  private initialization: Promise<void> | undefined;
  readonly draft = new SettingsDraft(
    defaultSettings,
    async (settings) =>
      (
        await workerCall<{ settings: CoreSettings }>("settings.update", {
          settings,
        })
      ).settings,
    () => this.publishDraft(),
  );
  subscribe = (fn: () => void) => {
    this.subscribers.add(fn);
    return () => {
      this.subscribers.delete(fn);
    };
  };
  getSnapshot = () => this.state;
  update(patch: Partial<AppState>) {
    this.state = { ...this.state, ...patch };
    for (const fn of this.subscribers) fn();
  }
  private publishDraft() {
    this.update({
      settings: this.draft.values,
      saveError: this.draft.error,
      saving: this.draft.saving,
      dirty: this.draft.dirty,
    });
  }
  applySnapshot(snapshot: Snapshot) {
    const ids = new Set(snapshot.jobs.map((job) => job.id));
    this.update({
      snapshot,
      selected: new Set([...this.state.selected].filter((id) => ids.has(id))),
      active:
        this.state.active && ids.has(this.state.active)
          ? this.state.active
          : null,
    });
    this.draft.apply(snapshot.settings);
  }
  event(event: WorkerEvent) {
    if (event.name === "snapshot_changed")
      this.applySnapshot(event.body as unknown as Snapshot);
    if (event.name === "core_health" && this.state.snapshot)
      this.update({
        snapshot: {
          ...this.state.snapshot,
          app: event.body as unknown as Snapshot["app"],
        },
      });
  }
  initialize() {
    return (this.initialization ??= (async () => {
      const queued: Array<() => void> = [];
      let buffering = true;
      const deliver = (fn: () => void) => (buffering ? queued.push(fn) : fn());
      try {
        await listen<WorkerEvent>("worker-event", (event) =>
          deliver(() => this.event(event.payload)),
        );
        await listen<string>("worker-log", (event) => {
          const buffered = buffering;
          deliver(() => {
            if (!buffered || !this.state.logs.includes(event.payload))
              this.update({
                logs: [event.payload, ...this.state.logs].slice(0, 300),
              });
          });
        });
        await listen<WorkerState>("worker-state", (event) =>
          deliver(() => this.update({ worker: event.payload })),
        );
        const data = await bootstrap();
        this.update({
          loaded: true,
          cookies: data.cookies,
          accounts: data.accounts,
          dependencies: data.dependencies,
          ui: data.ui_state,
          autostart: data.autostart_enabled,
          worker: data.worker_state,
          logs: data.logs,
          appRoot: data.app_root,
          dataRoot: data.data_root,
        });
        if (data.snapshot) this.applySnapshot(data.snapshot);
      } catch (error) {
        this.update({
          loaded: true,
          error: String(error),
          worker: { status: "disconnected", error: String(error) },
        });
      } finally {
        buffering = false;
        for (const fn of queued) fn();
      }
    })());
  }
  select(id: string, checked: boolean) {
    const selected = new Set(this.state.selected);
    if (checked) selected.add(id);
    else selected.delete(id);
    this.update({ selected });
  }
}
export const store = new AppStore();
export const useAppState = () =>
  useSyncExternalStore(store.subscribe, store.getSnapshot);
