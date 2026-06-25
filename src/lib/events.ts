import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Peer, TransferEvent } from "./types";

export function onPeerFound(cb: (peer: Peer) => void): Promise<UnlistenFn> {
  return listen<{ peer: Peer }>("peer://found", (e) => cb(e.payload.peer));
}
export function onPeerLost(cb: (device_id: string) => void): Promise<UnlistenFn> {
  return listen<{ device_id: string }>("peer://lost", (e) => cb(e.payload.device_id));
}
export function onTransferEvent(cb: (e: TransferEvent) => void): Promise<UnlistenFn> {
  return listen<TransferEvent>("transfer://request", (e) => cb(e.payload as TransferEvent))
    .then(async (u1) => {
      const u2 = await listen<TransferEvent>("transfer://progress", (e) => cb(e.payload as TransferEvent));
      const u3 = await listen<TransferEvent>("transfer://finished", (e) => cb(e.payload as TransferEvent));
      const combined: UnlistenFn = () => { u1(); u2(); u3(); };
      return combined;
    });
}
