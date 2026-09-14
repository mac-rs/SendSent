import { useEffect, useState } from "react";
import { getTransferConfig, setTransferConfig } from "../lib/invoke";

type Msg = { type: "ok" | "err"; text: string };

function Stepper({
  value,
  min,
  max,
  step = 1,
  onChange,
}: {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (v: number) => void;
}) {
  const dec = () => onChange(Math.max(min, value - step));
  const inc = () => onChange(Math.min(max, value + step));
  return (
    <div className="stepper">
      <button
        type="button"
        className="stepper-btn"
        onClick={dec}
        disabled={value <= min}
        aria-label="减少"
      >
        −
      </button>
      <input
        type="number"
        className="stepper-input"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => {
          const n = Number(e.target.value);
          if (Number.isFinite(n)) onChange(Math.max(min, Math.min(max, n)));
        }}
      />
      <button
        type="button"
        className="stepper-btn"
        onClick={inc}
        disabled={value >= max}
        aria-label="增加"
      >
        +
      </button>
    </div>
  );
}

export function TransferConfigEditor() {
  const [conns, setConns] = useState(8);
  const [chunkKb, setChunkKb] = useState(1024);
  const [splitMb, setSplitMb] = useState(4);
  const [zerocopy, setZerocopy] = useState(false);
  const [orig, setOrig] = useState<{ c: number; k: number; s: number; z: boolean } | null>(null);
  const [msg, setMsg] = useState<Msg | null>(null);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    getTransferConfig()
      .then((c) => {
        setConns(c.conns);
        setChunkKb(Math.round(c.chunk_size / 1024));
        setSplitMb(Math.round(c.split_threshold / (1024 * 1024)));
        setZerocopy(c.zerocopy);
        setOrig({ c: c.conns, k: Math.round(c.chunk_size / 1024), s: Math.round(c.split_threshold / (1024 * 1024)), z: c.zerocopy });
        setLoaded(true);
      })
      .catch(() => setLoaded(true));
  }, []);

  const dirty = loaded && !!orig && (
    conns !== orig.c || chunkKb !== orig.k || splitMb !== orig.s || zerocopy !== orig.z
  );

  const save = async () => {
    try {
      await setTransferConfig(conns, chunkKb, splitMb, zerocopy);
      setOrig({ c: conns, k: chunkKb, s: splitMb, z: zerocopy });
      setMsg({ type: "ok", text: "已保存" });
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
        <Stepper value={conns} min={1} max={32} onChange={setConns} />
      </div>

      <div className="config-row">
        <div className="name">
          数据块大小 (KB)
          <small>每个连接每次读写的块大小</small>
        </div>
        <Stepper value={chunkKb} min={64} max={1024} step={64} onChange={setChunkKb} />
      </div>

      <div className="config-row">
        <div className="name">
          分片阈值 (MB)
          <small>超过此大小的文件会被拆成多连接并行</small>
        </div>
        <Stepper value={splitMb} min={1} max={1024} onChange={setSplitMb} />
      </div>

      <div className="config-row">
        <div className="name">
          零拷贝传输
          <small>大文件走 sendfile 零拷贝,减少一次内存拷贝;仅明文路径生效,加密模式自动回退</small>
        </div>
        <label className="toggle">
          <input type="checkbox" checked={zerocopy} onChange={(e) => setZerocopy(e.target.checked)} />
          <span className="switch" />
        </label>
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
