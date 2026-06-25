import { respond, getDefaultSaveDir } from "../lib/invoke";
import type { RequestView } from "../hooks/useTransfer";

export function IncomingRequest({ req }: { req: RequestView | null }) {
  if (!req) return null;
  const r = req;
  const sizeMiB = (r.size / (1024 * 1024)).toFixed(1);
  async function accept() {
    const dir = await getDefaultSaveDir();
    await respond(r.session_id, true, dir);
  }
  async function reject() { await respond(r.session_id, false); }
  return (
    <div style={{ border: "1px solid #ccc", padding: 12 }}>
      <strong>{req.sender_name}</strong> 想发送 {req.count} 个文件({sizeMiB} MiB)
      <div style={{ marginTop: 8 }}>
        <button onClick={accept}>接受</button>
        <button onClick={reject} style={{ marginLeft: 8 }}>拒绝</button>
      </div>
    </div>
  );
}
