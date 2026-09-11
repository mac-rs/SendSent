// 持久化手动添加的 IP 列表(用户偏好)
// 最多保留 20 条,LRU 策略

const KEY = "sendsent.recent-peers";

export function getRecentPeers(): string[] {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return [];
    const arr = JSON.parse(raw);
    return Array.isArray(arr) ? arr.filter((s) => typeof s === "string") : [];
  } catch {
    return [];
  }
}

export function addRecentPeer(address: string) {
  const trimmed = address.trim();
  if (!trimmed) return;
  const list = getRecentPeers().filter((p) => p !== trimmed);
  list.unshift(trimmed);
  if (list.length > 20) list.length = 20;
  try {
    localStorage.setItem(KEY, JSON.stringify(list));
  } catch {
    // ignore quota or private mode
  }
}

export function removeRecentPeer(address: string) {
  const list = getRecentPeers().filter((p) => p !== address);
  try {
    localStorage.setItem(KEY, JSON.stringify(list));
  } catch {
    // ignore
  }
}

export function clearRecentPeers() {
  try {
    localStorage.removeItem(KEY);
  } catch {
    // ignore
  }
}
