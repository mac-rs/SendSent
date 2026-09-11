import { useCallback, useEffect, useState } from "react";
import { listPeers } from "../lib/invoke";
import { onPeerFound, onPeerLost } from "../lib/events";
import type { Peer } from "../lib/types";

export function usePeers() {
  const [peers, setPeers] = useState<Peer[]>([]);
  const [refreshing, setRefreshing] = useState(false);
  useEffect(() => {
    refresh();
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
  // 返回值:本次刷新发现的设备数(用于 ⌘R 后的 toast 反馈)
  const refresh = useCallback(async (): Promise<number> => {
    if (refreshing) return peers.length;
    setRefreshing(true);
    try {
      const fresh = await listPeers();
      setPeers(fresh);
      return fresh.length;
    } catch {
      return peers.length;
    } finally {
      setRefreshing(false);
    }
  }, [refreshing, peers.length]);
  return { peers, refreshing, refresh };
}
