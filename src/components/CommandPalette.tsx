// 命令面板 · ⌘⇧P 唤起
// 模糊搜索 + 键盘导航 + 命令分组

import { useEffect, useMemo, useRef, useState } from "react";
import { SearchIcon, XIcon } from "./Icons";

export type CommandItem = {
  id: string;
  label: string;
  hint?: string;
  group: string;
  keywords?: string[];
  shortcut?: string;
  action: () => void;
  icon?: React.ReactNode;
  disabled?: boolean;
};

export function CommandPalette({
  open,
  onClose,
  commands,
}: {
  open: boolean;
  onClose: () => void;
  commands: CommandItem[];
}) {
  const [q, setQ] = useState("");
  const [activeIdx, setActiveIdx] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) {
      setQ("");
      setActiveIdx(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") { e.preventDefault(); onClose(); return; }
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setActiveIdx((i) => Math.min(i + 1, filtered.length - 1));
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setActiveIdx((i) => Math.max(i - 1, 0));
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        const item = filtered[activeIdx];
        if (item && !item.disabled) {
          item.action();
          onClose();
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const filtered = useMemo(() => {
    const k = q.trim().toLowerCase();
    if (!k) return commands.filter((c) => !c.disabled);
    return commands
      .filter((c) => !c.disabled)
      .map((c) => {
        const haystack = [c.label, c.group, c.hint ?? "", ...(c.keywords ?? [])]
          .join(" ").toLowerCase();
        // 简单 fuzzy:每个字符顺序出现即可
        let i = 0;
        let score = 0;
        for (const ch of k) {
          const idx = haystack.indexOf(ch, i);
          if (idx === -1) return null;
          score += idx - i; // 越紧凑越好
          i = idx + 1;
        }
        return { c, score };
      })
      .filter((r): r is { c: CommandItem; score: number } => r !== null)
      .sort((a, b) => a.score - b.score)
      .map((r) => r.c);
  }, [q, commands]);

  // 当结果变化时重置 activeIdx
  useEffect(() => {
    if (activeIdx >= filtered.length) setActiveIdx(Math.max(0, filtered.length - 1));
  }, [filtered.length, activeIdx]);

  if (!open) return null;

  // 分组渲染
  const grouped: Record<string, CommandItem[]> = {};
  for (const c of filtered) {
    if (!grouped[c.group]) grouped[c.group] = [];
    grouped[c.group].push(c);
  }

  let itemIdx = -1;

  return (
    <div
      className="cmd-backdrop"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
      role="presentation"
    >
      <div className="cmd-panel" role="dialog" aria-modal aria-label="命令面板">
        <div className="cmd-search">
          <SearchIcon size={16} />
          <input
            ref={inputRef}
            type="text"
            placeholder="搜索命令…"
            value={q}
            onChange={(e) => { setQ(e.target.value); setActiveIdx(0); }}
            spellCheck={false}
            autoComplete="off"
          />
          {q && (
            <button
              className="cmd-clear"
              onClick={() => setQ("")}
              aria-label="清空"
            >
              <XIcon size={12} />
            </button>
          )}
        </div>
        <div className="cmd-results" role="listbox">
          {filtered.length === 0 ? (
            <div className="cmd-empty">没有匹配的命令</div>
          ) : (
            Object.entries(grouped).map(([group, items]) => (
              <div key={group} className="cmd-group">
                <div className="cmd-group-label">{group}</div>
                {items.map((c) => {
                  itemIdx++;
                  const isActive = itemIdx === activeIdx;
                  return (
                    <button
                      key={c.id}
                      className={`cmd-item${isActive ? " active" : ""}`}
                      onClick={() => { c.action(); onClose(); }}
                      onMouseEnter={() => setActiveIdx(itemIdx)}
                      role="option"
                      aria-selected={isActive}
                    >
                      {c.icon && <span className="cmd-item-icon">{c.icon}</span>}
                      <span className="cmd-item-main">
                        <span className="cmd-item-label">{c.label}</span>
                        {c.hint && <span className="cmd-item-hint">{c.hint}</span>}
                      </span>
                      {c.shortcut && <kbd className="cmd-item-kbd">{c.shortcut}</kbd>}
                    </button>
                  );
                })}
              </div>
            ))
          )}
        </div>
        <div className="cmd-footer">
          <span><kbd className="kbd">↑↓</kbd> 移动</span>
          <span><kbd className="kbd">↵</kbd> 执行</span>
          <span><kbd className="kbd">esc</kbd> 关闭</span>
        </div>
      </div>
    </div>
  );
}
