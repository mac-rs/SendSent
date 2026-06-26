import { useEffect, useState } from "react";
import { getTransferConfig, setTransferConfig } from "../lib/invoke";

export function TransferConfigEditor() {
  const [conns, setConns] = useState(8);
  const [chunkKb, setChunkKb] = useState(1024);
  const [splitMb, setSplitMb] = useState(4);
  const [open, setOpen] = useState(false);
  const [msg, setMsg] = useState("");

  useEffect(() => {
    getTransferConfig().then((c) => {
      setConns(c.conns);
      setChunkKb(Math.round(c.chunk_size / 1024));
      setSplitMb(Math.round(c.split_threshold / (1024 * 1024)));
    }).catch(() => {});
  }, []);

  const save = async () => {
    try {
      await setTransferConfig(conns, chunkKb, splitMb);
      setMsg("已保存,重启后生效");
    } catch (e: any) { setMsg("保存失败: " + String(e)); }
  };

  return (
    <div className="config-section">
      <div className="config-header" onClick={() => setOpen(!open)}>
        <span>传输参数</span>
        <span className="config-arrow">{open ? "▾" : "▸"}</span>
      </div>
      {open && (
        <div className="config-body">
          <label>并发连接 <input type="number" value={conns} min={1} max={32} onChange={(e) => setConns(Number(e.target.value))} /></label>
          <label>块大小 KB <input type="number" value={chunkKb} min={64} max={1024} step={64} onChange={(e) => setChunkKb(Number(e.target.value))} /></label>
          <label>拆分阈值 MB <input type="number" value={splitMb} min={1} onChange={(e) => setSplitMb(Number(e.target.value))} /></label>
          <button onClick={save}>保存</button>
          {msg && <span className="config-msg">{msg}</span>}
        </div>
      )}
    </div>
  );
}
