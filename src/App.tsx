import { useState } from "react";
import { usePeers } from "./hooks/usePeers";
import { useTransfer } from "./hooks/useTransfer";
import { PeerList } from "./components/PeerList";
import { FilePicker } from "./components/FilePicker";
import { IncomingRequest } from "./components/IncomingRequest";
import { TransferProgress } from "./components/TransferProgress";
import { Settings } from "./components/Settings";
import { addPeer } from "./lib/invoke";
import { sendText } from "./lib/invoke";
import type { Peer } from "./lib/types";
import "./App.css";

function App() {
  const peers = usePeers();
  const { request, progress } = useTransfer();
  const [selected, setSelected] = useState<Peer[]>([]);
  const togglePeer = (p: Peer) => setSelected((cur) =>
    cur.some((x) => x.device_id === p.device_id)
      ? cur.filter((x) => x.device_id !== p.device_id)
      : [...cur, p]
  );
  const [manualAddr, setManualAddr] = useState("");
  const [textContent, setTextContent] = useState("");
  const [textBusy, setTextBusy] = useState(false);
  return (
    <main className="container">
      <h1>SendSent</h1>
      <PeerList peers={peers} selected={selected} onToggle={togglePeer} />
      <div style={{ marginTop: 8 }}>
        <input
          placeholder="手动添加 IP:port"
          value={manualAddr}
          onChange={(e) => setManualAddr(e.target.value)}
          style={{ width: 180 }}
        />
        <button
          onClick={async () => {
            if (!manualAddr) return;
            try {
              await addPeer(manualAddr);
              setManualAddr("");
            } catch (e) { alert("添加失败: " + String(e)); }
          }}
          style={{ marginLeft: 6 }}
        >添加</button>
      </div>
      <div style={{ marginTop: 12 }}><FilePicker peers={selected} /></div>
      <div style={{ marginTop: 12 }}>
        <textarea
          rows={2}
          placeholder="发送文本…"
          value={textContent}
          onChange={(e) => setTextContent(e.target.value)}
          style={{ width: 260 }}
          disabled={selected.length === 0 || textBusy}
        />
        <div>
          <button
            disabled={selected.length === 0 || !textContent.trim() || textBusy}
            onClick={async () => {
              if (selected.length === 0 || !textContent.trim()) return;
              setTextBusy(true);
              try {
                for (const p of selected) { await sendText(p.device_id, textContent); }
                setTextContent("");
              }
              catch (e) { alert("发送失败: " + String(e)); }
              finally { setTextBusy(false); }
            }}
          >发送文本{selected.length > 0 ? ` → ${selected.map(p=>p.name).join(", ")}` : ""}</button>
        </div>
      </div>
      <div style={{ marginTop: 12 }}><IncomingRequest req={request} /></div>
      <div style={{ marginTop: 12 }}><TransferProgress items={Object.values(progress)} /></div>
      <Settings />
    </main>
  );
}
export default App;
