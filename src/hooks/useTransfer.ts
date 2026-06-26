import { useEffect, useState } from "react";
import { onTransferEvent } from "../lib/events";
import type { Manifest } from "../lib/types";

export interface RequestView {
  session_id: string; sender_name: string; count: number; size: number;
}
export interface ProgressView {
  session_id: string; bytes_done: number; bytes_total: number;
  files_done: number; files_total: number; speed_bps: number; done: boolean; error?: string;
  filenames?: string[]; started_at?: number; ended_at?: number;
}

// 模块级存储:发送端文件名(发送方没有 Request 事件,通过此 Map 补名字)
const sendFilenames = new Map<string, string[]>();
export function registerSendFilenames(sessionId: string, names: string[]) {
  sendFilenames.set(sessionId, names);
}

export function useTransfer() {
  const [request, setRequest] = useState<RequestView | null>(null);
  const [progress, setProgress] = useState<Record<string, ProgressView>>({});
  const [manifests] = useState<Record<string, Manifest>>({});

  useEffect(() => {
    let un: (() => void) | undefined;
    (async () => {
      un = await onTransferEvent((ev) => {
        if (ev.kind === "Request") {
          manifests[ev.session_id] = ev.manifest;
          setRequest({
            session_id: ev.session_id,
            sender_name: ev.sender.name,
            count: ev.manifest.total_count,
            size: ev.manifest.total_size,
          });
        } else if (ev.kind === "Progress") {
          setProgress((cur) => {
            const prev = cur[ev.session_id];
            const fromManifest = manifests[ev.session_id]?.files
              ?.filter((f) => f.kind === "File")
              .map((f) => f.name) ?? [];
            const fromSender = sendFilenames.get(ev.session_id) ?? [];
            const names = fromManifest.length > 0 ? fromManifest : fromSender;
            return {
              ...cur,
              [ev.session_id]: {
                session_id: ev.session_id, bytes_done: ev.bytes_done, bytes_total: ev.bytes_total,
                files_done: ev.files_done, files_total: ev.files_total, speed_bps: ev.speed_bps, done: false,
                filenames: names.length > 0 ? names : prev?.filenames,
                started_at: prev?.started_at ?? Date.now(),
              },
            };
          });
        } else {
          setProgress((cur) => {
            const prev = cur[ev.session_id];
            return {
              ...cur,
              [ev.session_id]: {
                session_id: ev.session_id,
                bytes_done: prev?.bytes_done ?? 0,
                bytes_total: prev?.bytes_total ?? 0,
                files_done: prev?.files_done ?? 0,
                files_total: prev?.files_total ?? 0,
                speed_bps: 0,
                done: true,
                error: ev.state !== "completed" ? ev.state : undefined,
                filenames: prev?.filenames,
                started_at: prev?.started_at,
                ended_at: Date.now(),
              },
            };
          });
          setRequest((r) => (r && r.session_id === ev.session_id ? null : r));
        }
      });
    })();
    return () => { un?.(); };
  }, []);

  return { request, progress, clearRequest: () => setRequest(null) };
}
