// 共享的设置内容,Settings tab 与 Cmd+, 偏好面板都引用

import { useEffect, useState } from "react";
import { getIdentity, setDisplayName } from "../lib/invoke";
import { SaveLocation } from "./SaveLocation";

type Msg = { type: "ok" | "err"; text: string };

export function SettingsForm() {
  const [name, setName] = useState("");
  const [saved, setSaved] = useState("");
  const [msg, setMsg] = useState<Msg | null>(null);

  useEffect(() => {
    getIdentity()
      .then((i) => { setName(i.name); setSaved(i.name); })
      .catch(() => {});
  }, []);

  const dirty = name.trim() !== "" && name !== saved;

  const save = async () => {
    if (!dirty) return;
    try {
      await setDisplayName(name.trim());
      setSaved(name.trim());
      setMsg({ type: "ok", text: "已保存,重启应用后生效" });
    } catch (e) {
      setMsg({ type: "err", text: "保存失败: " + String(e) });
    }
  };

  return (
    <div className="col">
      <div className="field">
        <label className="field-label" htmlFor="display-name">本机显示名</label>
        <div className="field-row">
          <input
            id="display-name"
            value={name}
            onChange={(e) => { setName(e.target.value); setMsg(null); }}
            placeholder="你的设备名"
            onKeyDown={(e) => { if (e.key === "Enter") save(); }}
          />
          <button
            className="btn btn-primary"
            disabled={!dirty}
            onClick={save}
          >
            保存
          </button>
        </div>
        <div className="field-hint">
          其他设备会看到这个名字,长度建议 2-20 个字符
        </div>
      </div>

      <SaveLocation />

      <div className="field">
        <label className="field-label">关于</label>
        <div className="field-hint" style={{ lineHeight: 1.6 }}>
          SendSent · 局域网 P2P 文件传输<br />
          基于 Tauri 2 + React 19,支持 macOS / iOS / Android / Windows / Linux。
        </div>
      </div>

      {msg && (
        <div className={`toast-msg${msg.type === "err" ? " error" : ""}`}>
          {msg.text}
        </div>
      )}
    </div>
  );
}
