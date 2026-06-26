import { useEffect, useState } from "react";
import { getTransferConfig, setTransferConfig } from "../lib/invoke";

type Msg = { type: "ok" | "err"; text: string };

export function TransferConfigEditor() {
  const [conns, setConns] = useState(8);
  const [chunkKb, setChunkKb] = useState(1024);
  const [splitMb, setSplitMb] = useState(4);
  const [msg, setMsg] = useState<Msg | null>(null);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    getTransferConfig()
      .then((c) => {
        setConns(c.conns);
        setChunkKb(Math.round(c.chunk_size / 1024));
        setSplitMb(Math.round(c.split_threshold / (1024 * 1024)));
        setLoaded(true);
      })
      .catch(() => setLoaded(true));
  }, []);

  const dirty = loaded && (
    conns !== 8 || chunkKb !== 1024 || splitMb !== 4
  );

  const save = async () => {
    try {
      await setTransferConfig(conns, chunkKb, splitMb);
      setMsg({ type: "ok", text: "已保存,重启应用后生效" });
    } catch (e) {
      setMsg({ type: "err", text: "保存失败: " + String(e) });
    }
  };

  return (
    <div className="col">
      <div className="config-row">
        <div className="name">
          并发连接数
          <small>多连接传输可显著提高大文件速度</small>
        </div>
        <input
          type="number"
          min={1}
          max={32}
          value={conns}
          onChange={(e) => setConns(Number(e.target.value) || 1)}
        />
      </div>

      <div className="config-row">
        <div className="name">
          数据块大小 (KB)
          <small>每个连接每次读写的块大小</small>
        </div>
        <input
          type="number"
          min={64}
          max={1024}
          step={64}
          value={chunkKb}
          onChange={(e) => setChunkKb(Number(e.target.value) || 64)}
        />
      </div>

      <div className="config-row">
        <div className="name">
          分片阈值 (MB)
          <small>超过此大小的文件会被拆成多连接并行</small>
        </div>
        <input
          type="number"
          min={1}
          value={splitMb}
          onChange={(e) => setSplitMb(Number(e.target.value) || 1)}
        />
      </div>

      <div className="row" style={{ justifyContent: "flex-end" }}>
        <button
          className="btn btn-primary"
          disabled={!dirty}
          onClick={save}
        >
          保存设置
        </button>
      </div>

      {msg && (
        <div className={`toast-msg${msg.type === "err" ? " error" : ""}`}>
          {msg.text}
        </div>
      )}
    </div>
  );
}
