import { useState, useEffect } from "react";
import { usePeers } from "./hooks/usePeers";
import { useTransfer } from "./hooks/useTransfer";
import { PeerList } from "./components/PeerList";
import { FilePicker } from "./components/FilePicker";
import { IncomingRequest } from "./components/IncomingRequest";
import { TransferProgress } from "./components/TransferProgress";
import { addPeer, sendText, respond } from "./lib/invoke";
import type { Peer } from "./lib/types";

function App() {
  const peers = usePeers();
  const { request, progress, clearRequest } = useTransfer();
  const [selected, setSelected] = useState<Peer[]>([]);
  const [dark, setDark] = useState(false);
  const [manualAddr, setManualAddr] = useState("");
  const [textContent, setTextContent] = useState("");
  const [textBusy, setTextBusy] = useState(false);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", dark);
  }, [dark]);

  const togglePeer = (p: Peer) =>
    setSelected((c) =>
      c.some((x) => x.device_id === p.device_id)
        ? c.filter((x) => x.device_id !== p.device_id)
        : [...c, p]
    );

  return (
    <main className="app-container">
      {/* Header + Theme Toggle */}
      <header className="app-header">
        <span className="app-title">SendSent</span>
        <div className="theme-toggle" onClick={() => setDark(!dark)}>
          <span>{dark ? "🌙" : "☀️"}</span>
          <div className={`theme-track${dark ? " active" : ""}`}>
            <div className="theme-thumb" />
          </div>
        </div>
      </header>

      {/* Peer list */}
      <PeerList peers={peers} selected={selected} onToggle={togglePeer} />

      {/* Manual IP */}
      <div className="add-peer-row">
        <input
          placeholder="手动添加 IP:port"
          value={manualAddr}
          onChange={(e) => setManualAddr(e.target.value)}
        />
        <button
          onClick={async () => {
            if (!manualAddr) return;
            try { await addPeer(manualAddr); setManualAddr(""); }
            catch (e) { alert("失败: " + String(e)); }
          }}
        >添加</button>
      </div>

      {/* File picker */}
      <FilePicker peers={selected} />

      {/* Text send */}
      <div className="text-row">
        <textarea
          rows={2}
          placeholder="或输入文字发送…"
          value={textContent}
          onChange={(e) => setTextContent(e.target.value)}
          disabled={selected.length === 0 || textBusy}
        />
        <button
          disabled={selected.length === 0 || !textContent.trim() || textBusy}
          onClick={async () => {
            if (selected.length === 0 || !textContent.trim()) return;
            setTextBusy(true);
            try { for (const p of selected) await sendText(p.device_id, textContent); setTextContent(""); }
            catch (e) { alert("失败: " + String(e)); }
            finally { setTextBusy(false); }
          }}
        >发送</button>
      </div>

      {/* Progress */}
      <TransferProgress items={Object.values(progress)} />

      {/* Incoming request */}
      <IncomingRequest req={request} onRespond={(accept) => {
        if (request) respond(request.session_id, accept);
        clearRequest();
      }} />

      {/* Settings */}
      <SettingsRow />
    </main>
  );
}

function SettingsRow() {
  const [name, setName] = useState("");
  useEffect(() => {
    import("./lib/invoke").then(({ getIdentity }) => {
      getIdentity().then((i) => setName(i.name)).catch(() => {});
    });
  }, []);
  return (
    <div className="settings-row">
      <input
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="本机显示名"
      />
      <button onClick={() => import("./lib/invoke").then(({ setDisplayName }) => setDisplayName(name))}>
        保存
      </button>
    </div>
  );
}

export default App;
