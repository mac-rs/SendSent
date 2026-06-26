import { respond } from "../lib/invoke";
import type { RequestView } from "../hooks/useTransfer";

export function IncomingRequest({ req }: { req: RequestView | null }) {
  if (!req) return null;
  const sizeMiB = (req.size / (1024 * 1024)).toFixed(1);
  return (
    <div className="request-backdrop" onClick={(e) => { if (e.target === e.currentTarget) respond(req.session_id, false); }}>
      <div className="request-card">
        <h3>{req.sender_name}</h3>
        <div className="meta">{req.count} 个文件 · {sizeMiB} MiB</div>
        <div className="actions">
          <button className="btn-reject" onClick={() => respond(req.session_id, false)}>拒绝</button>
          <button className="btn-accept" onClick={() => respond(req.session_id, true, undefined)}>接受</button>
        </div>
      </div>
    </div>
  );
}
