import { useEffect, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import { getDefaultSaveDir } from "../lib/invoke";
import { usePlatform } from "../lib/platform";

export function SaveLocation() {
  const { platform } = usePlatform();
  const [dir, setDir] = useState("");
  const [msg, setMsg] = useState("");

  useEffect(() => {
    getDefaultSaveDir().then(setDir).catch((e) => setMsg(String(e)));
  }, []);

  const isMobile = platform === "ios" || platform === "android";

  return (
    <div className="field">
      <label className="field-label">保存位置</label>
      <div className="field-row">
        <input readOnly value={dir} />
        {!isMobile && (
          <button
            className="btn btn-primary"
            disabled={!dir}
            onClick={() => openPath(dir).catch((e) => setMsg(String(e)))}
          >
            打开文件夹
          </button>
        )}
      </div>
      <div className="field-hint">{isMobile ? "接收的文件保存在此目录" : "接收的文件默认保存于此"}</div>
      {msg && <div className="toast-msg error">{msg}</div>}
    </div>
  );
}
