import { useEffect, useState } from "react";
import { listPeers } from "../lib/invoke";
import { onPeerFound, onPeerLost } from "../lib/events";
import type { Peer } from "../lib/types";

export function usePeers() {
  const [peers, setPeers] = useState<Peer[]>([]);
  useEffect(() => {
    listPeers().then(setPeers).catch(() => {});
    let un1: (() => void) | undefined;
    let un2: (() => void) | undefined;
    (async () => {
      un1 = await onPeerFound((p) =>
        setPeers((cur) => (cur.some((x) => x.device_id === p.device_id) ? cur : [...cur, p])));
      un2 = await onPeerLost((id) =>
        setPeers((cur) => cur.filter((x) => x.device_id !== id)));
    })();
    return () => { un1?.(); un2?.(); };
  }, []);
  return peers;
}
