import { useState } from "react";
import { store, useAppState } from "../lib/store";
import { setAutostart } from "../lib/desktop";
import {
  recordingSections,
  notificationSections,
  appearanceSections,
  type SettingSection,
} from "../lib/settings";
import Field from "../components/Field";
import CredentialsEditor from "../components/CredentialsEditor";
export default function SettingsPage() {
  const state = useAppState();
  const [tab, setTab] = useState("recording");
  const [showSecrets, setShowSecrets] = useState(false);
  const [starting, setStarting] = useState(false);
  const connected = state.worker.status === "connected";
  function sections(groups: SettingSection[]) {
    return groups.map((group) => (
      <section className="setting-section" key={group.title}>
        <div>
          <h2>{group.title}</h2>
          {group.description && <p>{group.description}</p>}
        </div>
        <div className="setting-fields">
          {group.fields.map((field) => (
            <Field
              key={field.key}
              label={field.label}
              value={state.settings[field.key] as string | boolean}
              options={field.options}
              secret={field.secret && !showSecrets}
              hint={field.hint}
              disabled={!connected}
              onChange={(value) => store.draft.set(field.key, value)}
            />
          ))}
        </div>
      </section>
    ));
  }
  async function autostart(value: string | boolean) {
    setStarting(true);
    try {
      await setAutostart(Boolean(value));
      store.update({ autostart: Boolean(value) });
    } catch (error) {
      store.update({ error: String(error) });
    } finally {
      setStarting(false);
    }
  }
  return (
    <section>
      <header className="page-header">
        <div>
          <h1>设置</h1>
          <p>录制和推送设置自动保存。</p>
        </div>
        <span
          className={`save-status ${state.saveError ? "failed" : ""}`}
          role="status"
        >
          {state.saveError
            ? `自动保存失败：${state.saveError}`
            : state.saving
              ? "正在保存…"
              : state.dirty
                ? "等待保存…"
                : "修改后自动保存"}
        </span>
      </header>
      <nav className="settings-nav" role="tablist">
        {[
          ["recording", "录制"],
          ["notifications", "推送"],
          ["appearance", "外观与启动"],
          ["cookies", "Cookies"],
          ["accounts", "账号"],
        ].map(([key, title]) => (
          <button
            role="tab"
            aria-selected={tab === key}
            key={key}
            onClick={() => setTab(key)}
          >
            {title}
          </button>
        ))}
      </nav>
      <div className="settings-layout" hidden={tab !== "recording"}>
        {sections(recordingSections)}
      </div>
      <div className="settings-layout" hidden={tab !== "notifications"}>
        <label className="check-field">
          <input
            type="checkbox"
            checked={showSecrets}
            onChange={(event) => setShowSecrets(event.target.checked)}
          />
          显示敏感内容
        </label>
        {sections(notificationSections)}
      </div>
      <div className="settings-layout" hidden={tab !== "appearance"}>
        {sections(appearanceSections)}
        <section className="setting-section">
          <div>
            <h2>开机启动</h2>
            <p>登录 Windows 后启动到托盘，不显示主窗口。</p>
          </div>
          <Field
            label="开机自动启动"
            value={state.autostart}
            disabled={starting || !state.loaded}
            onChange={(value) => void autostart(value)}
          />
        </section>
      </div>
      <div hidden={tab !== "cookies"}>
        <CredentialsEditor kind="cookies" />
      </div>
      <div hidden={tab !== "accounts"}>
        <CredentialsEditor kind="accounts" />
      </div>
    </section>
  );
}
