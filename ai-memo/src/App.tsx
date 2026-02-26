import { useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

// 备忘录数据结构（包含是否完成、是否处于编辑/滑出状态等）
type MemoItem = {
  id: string;
  text: string;
  completed: boolean;
  // 下方为 UI 状态字段（不参与持久化）
  _revealed?: boolean; // 是否已左滑露出删除
};

// 生成简单唯一 ID（演示用途）
const uid = () => Math.random().toString(36).slice(2) + Date.now().toString(36);

export default function App() {
  const [input, setInput] = useState('');
  const [memos, setMemos] = useState<MemoItem[]>([]);

  // 编辑态
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingText, setEditingText] = useState('');

  // 拖拽态（指针事件实现，支持长按开启）
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const longPressTimer = useRef<number | null>(null);
  const listRef = useRef<HTMLUListElement | null>(null);
  const itemRefs = useRef<Map<string, HTMLLIElement>>(new Map());

  // 触摸滑动删除阈值
  const REVEAL_THRESHOLD = 64; // 左滑超过该像素则固定露出删除

  const isTauri = Boolean((window as any).__TAURI_INTERNALS__);

  // 安全调用 Tauri IPC（失败即返回 null，用于优雅降级）
  const safeInvoke = async <T,>(cmd: string, args?: Record<string, any>): Promise<T | null> => {
    try { return await invoke<T>(cmd, args); } catch (e) { return null; }
  };

  // 组件挂载时：若在 Tauri 环境，则从后端读取持久化数据
  useEffect(() => {
    (async () => {
      if (isTauri) {
        const data = await safeInvoke<MemoItem[]>('get_memos');
        if (data) setMemos(data);
      } else {
        // 非 Tauri：尝试调用后端 HTTP API（容器环境下可用）
        try {
          const res = await fetch('/api/memos');
          if (res.ok) {
            const json = await res.json();
            setMemos(json as MemoItem[]);
          }
        } catch (e) {
          // 无后端时保持空列表（纯前端预览）
        }
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 添加备忘录
  const handleAddMemo = async () => {
    const value = input.trim();
    if (!value) return;

    if (isTauri) {
      // 走 IPC：由后端生成 id、存盘并返回最新列表
      const updated = await safeInvoke<MemoItem[]>('add_memo', { text: value });
      if (updated) { setMemos(updated); setInput(''); return; }
    } else {
      // 非 Tauri：调用 HTTP API
      try {
        const res = await fetch('/api/memos', {
          method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ text: value })
        });
        if (res.ok) { const json = await res.json(); setMemos(json as MemoItem[]); setInput(''); return; }
      } catch (e) {}
    }

    // 纯前端兜底
    setMemos((prev) => [...prev, { id: uid(), text: value, completed: false }]);
    setInput('');
  };

  // 勾选完成/反选
  const toggleComplete = async (id: string) => {
    if (isTauri) {
      const updated = await safeInvoke<MemoItem[]>('toggle_complete', { id });
      if (updated) { setMemos(updated); return; }
    } else {
      try {
        const res = await fetch('/api/memos/toggle', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ id }) });
        if (res.ok) { const json = await res.json(); setMemos(json as MemoItem[]); return; }
      } catch (e) {}
    }
    setMemos((prev) => prev.map(m => m.id === id ? { ...m, completed: !m.completed } : m));
  };

  // 删除
  const deleteMemo = async (id: string) => {
    if (isTauri) {
      const updated = await safeInvoke<MemoItem[]>('delete_memo', { id });
      if (updated) { setMemos(updated); return; }
    } else {
      try {
        const res = await fetch(`/api/memos/${id}`, { method: 'DELETE' });
        if (res.ok) { const json = await res.json(); setMemos(json as MemoItem[]); return; }
      } catch (e) {}
    }
    setMemos((prev) => prev.filter(m => m.id !== id));
  };

  // 开始编辑
  const startEdit = (id: string, currentText: string) => {
    setEditingId(id);
    setEditingText(currentText);
  };

  // 保存编辑
  const saveEdit = async () => {
    if (!editingId) return;
    const value = editingText.trim();
    if (!value) { setEditingId(null); return; }
    if (isTauri) {
      const updated = await safeInvoke<MemoItem[]>('edit_memo', { id: editingId, text: value });
      if (updated) { setMemos(updated); setEditingId(null); setEditingText(''); return; }
    } else {
      try {
        const res = await fetch(`/api/memos/${editingId}`, { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ text: value }) });
        if (res.ok) { const json = await res.json(); setMemos(json as MemoItem[]); setEditingId(null); setEditingText(''); return; }
      } catch (e) {}
    }
    setMemos((prev) => prev.map(m => m.id === editingId ? { ...m, text: value } : m));
    setEditingId(null);
    setEditingText('');
  };

  // 取消编辑
  const cancelEdit = () => {
    setEditingId(null);
    setEditingText('');
  };

  // 记录某项的 DOM 引用
  const setItemRef = (id: string) => (el: HTMLLIElement | null) => {
    if (!el) {
      itemRefs.current.delete(id);
    } else {
      itemRefs.current.set(id, el);
    }
  };

  // 长按开启拖拽（pointer 事件）
  const onItemPointerDown = (e: React.PointerEvent, id: string) => {
    // 忽略右键
    if (e.button === 2) return;
    // 在移动端/触摸场景下，长按 200ms 开始拖拽
    if (longPressTimer.current) window.clearTimeout(longPressTimer.current);
    // @ts-ignore - setTimeout 返回 number 浏览器环境
    longPressTimer.current = window.setTimeout(() => {
      setDraggingId(id);
      (e.target as HTMLElement).setPointerCapture?.(e.pointerId);
      document.body.style.userSelect = 'none'; // 拖拽中禁用选中文本
    }, 200);
  };

  const onItemPointerMove = (e: React.PointerEvent) => {
    if (!draggingId) return;
    e.preventDefault();
    // 根据指针的 Y 坐标判断应当插入到哪个索引
    const listEl = listRef.current;
    if (!listEl) return;
    const rects = memos.map(m => {
      const el = itemRefs.current.get(m.id);
      const r = el?.getBoundingClientRect();
      return { id: m.id, top: r?.top ?? 0, bottom: r?.bottom ?? 0, mid: r ? (r.top + r.bottom) / 2 : 0 };
    });
    const clientY = e.clientY;
    // 找到第一个 mid 大于当前 Y 的项，作为插入点
    let targetIndex = rects.findIndex(r => clientY < r.mid);
    if (targetIndex === -1) targetIndex = rects.length; // 末尾

    const fromIndex = memos.findIndex(m => m.id === draggingId);
    if (fromIndex === -1) return;
    if (targetIndex === fromIndex || targetIndex === fromIndex + 1) return; // 没有跨越相邻边界

    setMemos((prev) => {
      const curIndex = prev.findIndex(m => m.id === draggingId);
      if (curIndex === -1) return prev;
      const clone = prev.slice();
      const [m] = clone.splice(curIndex, 1);
      // 调整 targetIndex（如果从前面移除，目标索引减 1）
      const adj = targetIndex > curIndex ? targetIndex - 1 : targetIndex;
      clone.splice(adj, 0, m);
      return clone;
    });
  };

  const onItemPointerUp = async (_e: React.PointerEvent) => {
    if (longPressTimer.current) window.clearTimeout(longPressTimer.current);
    if (draggingId) {
      setDraggingId(null);
      document.body.style.userSelect = '';
      // 拖拽结束后，将当前顺序同步到后端，确保多页面/刷新一致
      const order = memos.map(m => m.id);
      if (isTauri) {
        const updated = await safeInvoke<MemoItem[]>('reorder_memos', { order });
        if (updated) setMemos(updated);
      } else {
        try {
          const res = await fetch('/api/memos/reorder', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ order }) });
          if (res.ok) { const json = await res.json(); setMemos(json as MemoItem[]); }
        } catch (e) {}
      }
    }
  };

  // 触摸左右滑动以露出删除按钮
  const touchStartX = useRef<number>(0);
  const activeSwipeId = useRef<string | null>(null);

  const onTouchStart = (e: React.TouchEvent, id: string) => {
    touchStartX.current = e.touches[0]?.clientX ?? 0;
    activeSwipeId.current = id;
  };

  const onTouchMove = (e: React.TouchEvent, id: string) => {
    if (activeSwipeId.current !== id || draggingId) return;
    const dx = (e.touches[0]?.clientX ?? 0) - touchStartX.current;
    // 仅处理左滑（dx < 0）
    const shouldReveal = dx < -REVEAL_THRESHOLD;
    setMemos(prev => prev.map(m => m.id === id ? { ...m, _revealed: shouldReveal } : { ...m, _revealed: false }));
  };

  const onTouchEnd = (_e: React.TouchEvent, _id: string) => {
    activeSwipeId.current = null;
  };

  // 单击进行再次编辑（若已露出删除则先收起）
  const onItemClick = (id: string, text: string) => {
    setMemos(prev => prev.map(m => m.id === id ? { ...m, _revealed: false } : m));
    startEdit(id, text);
  };

  const styles = useMemo(() => ({
    container: { padding: '20px', fontFamily: 'system-ui', maxWidth: '480px', margin: '0 auto' },
    title: { color: '#333' },
    row: { display: 'flex', gap: '10px' },
    input: { flex: 1, padding: '10px', borderRadius: '8px', border: '1px solid #ccc', fontSize: '16px' } as React.CSSProperties,
    btnPrimary: { padding: '10px 16px', borderRadius: '8px', background: '#0070f3', color: '#fff', border: 'none', fontWeight: 'bold' } as React.CSSProperties,
    list: { marginTop: '20px', paddingLeft: 0, listStyle: 'none' } as React.CSSProperties,
    li: (active: boolean) => ({
      position: 'relative',
      overflow: 'hidden',
      borderBottom: '1px solid #eee',
      background: '#fff',
      marginBottom: 8,
      borderRadius: 8,
      boxShadow: active ? '0 2px 10px rgba(0,0,0,0.12)' : 'none',
      touchAction: 'pan-y', // 允许纵向滚动，减少误触
    }) as React.CSSProperties,
    // 内容层（可左滑）
    itemContent: (revealed: boolean) => ({
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      padding: 12,
      background: revealed ? '#fff0f0' : '#f9f9f9',
      transform: revealed ? `translateX(-${REVEAL_THRESHOLD}px)` : 'translateX(0px)',
      transition: 'transform 180ms ease',
    }) as React.CSSProperties,
    // 删除按钮（露出时可点击）
    deleteZone: {
      position: 'absolute' as const,
      right: 0,
      top: 0,
      bottom: 0,
      width: REVEAL_THRESHOLD,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      background: '#ff3b30',
      color: '#fff',
      fontWeight: 700,
      userSelect: 'none' as const,
    },
    memoText: (done: boolean) => ({
      flex: 1,
      color: done ? '#999' : '#222',
      textDecoration: done ? 'line-through' : 'none',
      wordBreak: 'break-word' as const,
    }) as React.CSSProperties,
    editInput: { flex: 1, padding: '8px', borderRadius: 6, border: '1px solid #ccc', fontSize: 15 } as React.CSSProperties,
    env: { fontSize: 12, color: '#888', marginTop: 20 },
  }), [REVEAL_THRESHOLD]);

  // 动态检测 HTTP API 可达性，用于 Web 模式下显示 🟢
  const [httpAlive, setHttpAlive] = useState<boolean>(false);
  useEffect(() => {
    if (isTauri) return; // Tauri 下优先显示 IPC 状态
    (async () => {
      try {
        const res = await fetch('/api/memos', { method: 'GET' });
        setHttpAlive(res.ok);
      } catch {
        setHttpAlive(false);
      }
    })();
  }, [isTauri]);

  return (
    <div style={styles.container}>
      <h2 style={styles.title}>✨ React + Rust 跨端备忘录</h2>
      <div style={styles.row as any}>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="输入备忘录内容..."
          style={styles.input}
        />
        <button 
          onClick={handleAddMemo}
          style={styles.btnPrimary}
        >
          记录
        </button>
      </div>

      <ul ref={listRef} style={styles.list}>
        {memos.map((m) => {
          const isDragging = draggingId === m.id;
          const revealed = Boolean(m._revealed);
          const isEditing = editingId === m.id;
          return (
            <li
              key={m.id}
              ref={setItemRef(m.id)}
              style={styles.li(isDragging)}
              onPointerDown={(e) => onItemPointerDown(e, m.id)}
              onPointerMove={onItemPointerMove}
              onPointerUp={onItemPointerUp}
              onTouchStart={(e) => onTouchStart(e, m.id)}
              onTouchMove={(e) => onTouchMove(e, m.id)}
              onTouchEnd={(e) => onTouchEnd(e, m.id)}
            >
              {/* 删除按钮区域（在右侧固定） */}
              <div style={styles.deleteZone} onClick={() => deleteMemo(m.id)}>
                🗑️
              </div>

              {/* 可左滑的内容区域 */}
              <div style={styles.itemContent(revealed)}>
                {/* 左侧勾选完成开关，可随时切换 */}
                <input type="checkbox" checked={m.completed} onChange={() => toggleComplete(m.id)} />

                {/* 正常态：文本 + 点击进入编辑；编辑态：输入框保存 */}
                {isEditing ? (
                  <input
                    autoFocus
                    value={editingText}
                    onChange={(e) => setEditingText(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') saveEdit();
                      if (e.key === 'Escape') cancelEdit();
                    }}
                    onBlur={saveEdit}
                    placeholder="编辑备忘录..."
                    style={styles.editInput}
                  />
                ) : (
                  <div style={styles.memoText(m.completed)} onClick={() => onItemClick(m.id, m.text)}>
                    {m.text}
                  </div>
                )}
              </div>
            </li>
          );
        })}
      </ul>

      <p style={styles.env}>
        运行环境状态: {
          isTauri
            ? '🟢 已连接 Rust 底层（支持 IPC）'
            : (httpAlive ? '🟢 已连接后端（HTTP API）' : '🟡 纯 Web 预览模式（前端本地状态）')
        }
      </p>
    </div>
  );
}
