import type { RequestView } from "../hooks/useTransfer";

export function IncomingRequest({ req, onRespond }: { req: RequestView | null; onRespond: (accept: boolean) => void }) {
  if (!req) return null;
  const sizeMiB = (req.size / (1024 * 1024)).toFixed(1);
  return (
    <div className="request-backdrop" onClick={(e) => { if (e.target === e.currentTarget) onRespond(false); }}>
      <div className="request-card">
        <h3>{req.sender_name}</h3>
        <div className="meta">{req.count} 个文件 · {sizeMiB} MiB</div>
        <div className="actions">
          <button className="btn-reject" onClick={() => onRespond(false)}>拒绝</button>
          <button className="btn-accept" onClick={() => onRespond(true)}>接受</button>
        </div>
      </div>
    </div>
  );
}
