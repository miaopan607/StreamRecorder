import { useEffect, useState } from "react";
import { store, useAppState } from "../lib/store";
import { workerCall } from "../lib/desktop";
import { accountKeys, cookiePlatforms } from "../lib/settings";
import type { Json } from "../lib/types";
import Field from "./Field";
import Modal from "./Modal";
export default function CredentialsEditor({
  kind,
}: {
  kind: "cookies" | "accounts";
}) {
  const state = useAppState();
  const source = state[kind];
  const [texts, setTexts] = useState<Record<string, string>>({});
  const [kinds, setKinds] = useState<Record<string, "text" | "json">>({});
  const [dirty, setDirty] = useState(false);
  const [reloading, setReloading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [visible, setVisible] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const title = kind === "cookies" ? "Cookies" : "账号";
  useEffect(() => {
    if (dirty) return;
    const keys = [
      ...new Set([
        ...(kind === "cookies" ? cookiePlatforms : accountKeys),
        ...Object.keys(source),
      ]),
    ];
    const texts: Record<string, string> = {},
      kinds: Record<string, "text" | "json"> = {};
    for (const key of keys) {
      const value = Object.hasOwn(source, key) ? source[key] : "";
      kinds[key] = typeof value === "string" ? "text" : "json";
      texts[key] =
        typeof value === "string" ? value : JSON.stringify(value, null, 2);
    }
    setTexts(texts);
    setKinds(kinds);
  }, [source, kind, dirty]);
  async function save() {
    setError("");
    setNotice("");
    const values: Record<string, Json> = { ...source };
    try {
      for (const [key, text] of Object.entries(texts))
        values[key] = kinds[key] === "json" ? JSON.parse(text) : text;
    } catch (e) {
      setError(`账号 JSON 格式有误：${String(e)}`);
      return;
    }
    setBusy(true);
    try {
      if (kind === "cookies") {
        const response = await workerCall<{ cookies: Record<string, string> }>(
          "cookies.update",
          { cookies: values },
        );
        store.update({ cookies: response.cookies });
      } else {
        const response = await workerCall<{ accounts: Record<string, Json> }>(
          "accounts.update",
          { accounts: values },
        );
        store.update({ accounts: response.accounts });
      }
      setDirty(false);
      setNotice(`${title}已保存`);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function reload() {
    setReloading(false);
    setError("");
    setNotice("");
    setBusy(true);
    try {
      if (kind === "cookies") {
        const response = await workerCall<{ cookies: Record<string, string> }>(
          "cookies.get",
        );
        store.update({ cookies: response.cookies });
      } else {
        const response = await workerCall<{ accounts: Record<string, Json> }>(
          "accounts.get",
        );
        store.update({ accounts: response.accounts });
      }
      setDirty(false);
      setNotice(`${title}已重新加载`);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <div className="credential-toolbar">
        <label className="check-field">
          <input
            type="checkbox"
            checked={visible}
            onChange={(event) => setVisible(event.target.checked)}
          />
          显示敏感内容
        </label>
        <div className="inline-actions">
          <span className="muted">{dirty ? "有未保存的修改" : notice}</span>
          <button
            disabled={busy || state.worker.status !== "connected"}
            onClick={() => {
              if (dirty) setReloading(true);
              else void reload();
            }}
          >
            重新加载{title}
          </button>
          <button
            className="primary"
            disabled={busy || state.worker.status !== "connected"}
            onClick={() => void save()}
          >
            {busy ? "正在处理…" : `保存${title}`}
          </button>
        </div>
      </div>
      {error && (
        <div className="error-inline" role="alert">
          {error}
        </div>
      )}
      <div className="credentials">
        {Object.entries(texts)
          .sort(([a], [b]) => a.localeCompare(b))
          .map(([key, value]) => (
            <div className="credential-row" key={key}>
              <Field
                label={key}
                value={value}
                secret={
                  !visible &&
                  (kind === "cookies" || /password|token|secret/i.test(key))
                }
                multiline={kinds[key] === "json"}
                hint={
                  kinds[key] === "json" ? "JSON 数据，保留原有类型" : undefined
                }
                disabled={!state.loaded || busy}
                onChange={(value) => {
                  setTexts({ ...texts, [key]: String(value) });
                  setDirty(true);
                  setNotice("");
                }}
              />
            </div>
          ))}
      </div>
      {reloading && (
        <Modal title="放弃未保存的修改？" onClose={() => setReloading(false)}>
          <div className="modal-body">
            <p>重新加载会覆盖当前尚未保存的{title}修改。</p>
          </div>
          <footer className="modal-footer">
            <button onClick={() => setReloading(false)}>取消</button>
            <button className="primary" onClick={() => void reload()}>
              重新加载
            </button>
          </footer>
        </Modal>
      )}
    </>
  );
}
