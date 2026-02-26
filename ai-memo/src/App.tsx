import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export default function App() {
  const [input, setInput] = useState('');
  const [memos, setMemos] = useState<string[]>([]);

  const handleAddMemo = async () => {
    if (!input.trim()) return;
    
    try {
      // 尝试跨语言调用 Rust 后端
      const updatedMemos: string[] = await invoke('add_memo', { newMemo: input });
      setMemos(updatedMemos);
    } catch (error) {
      // 【优雅降级】如果在普通手机浏览器里预览，走纯前端逻辑
      console.warn("当前处于纯 Web 预览模式，Rust 后端未连接。使用纯前端状态。");
      setMemos(prev => [...prev, input]);
    } finally {
      setInput('');
    }
  };

  return (
    <div style={{ padding: '20px', fontFamily: 'system-ui', maxWidth: '400px', margin: '0 auto' }}>
      <h2 style={{ color: '#333' }}>✨ AI Agent 跨端备忘录</h2>
      <div style={{ display: 'flex', gap: '10px' }}>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="输入备忘录内容..."
          style={{ flex: 1, padding: '10px', borderRadius: '8px', border: '1px solid #ccc', fontSize: '16px' }}
        />
        <button 
          onClick={handleAddMemo} 
          style={{ padding: '10px 16px', borderRadius: '8px', background: '#0070f3', color: '#fff', border: 'none', fontWeight: 'bold' }}
        >
          记录
        </button>
      </div>
      
      <ul style={{ marginTop: '20px', paddingLeft: '0', listStyle: 'none' }}>
        {memos.map((memo, idx) => (
          <li key={idx} style={{ padding: '12px', borderBottom: '1px solid #eee', background: '#f9f9f9', marginBottom: '8px', borderRadius: '6px' }}>
            {memo}
          </li>
        ))}
      </ul>
      <p style={{ fontSize: '12px', color: '#888', marginTop: '20px' }}>
        运行环境状态: {window.__TAURI_INTERNALS__ ? '🟢 已连接 Rust 底层' : '🟡 纯 Web 预览模式'}
      </p>
    </div>
  );
}
