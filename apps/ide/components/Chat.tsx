import React, { useState } from 'react';
import { sendPrompt } from '../lib/api';
import { BitsStatus } from './BitsStatus';

export function Chat({ branchId, onResult }: { branchId: string; onResult: (r: any) => void }) {
  const [text, setText] = useState('');
  const [busy, setBusy] = useState(false);
  const [bits, setBits] = useState<any | undefined>(undefined);

  async function onSend(e?: React.FormEvent) {
    e?.preventDefault();
    if (!text.trim() || busy) return;
    setBusy(true);
    setBits(undefined);
    try {
      const res = await sendPrompt(branchId, text.trim());
      setText('');
      setBits(res?.bits || res?.status?.bits);
      onResult(res);
    } catch (err) {
      onResult({ error: String(err) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="card">
      <form onSubmit={onSend} className="row">
        <input
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="Ask or instruct the engine..."
          style={{ flex: 1, padding: 8, borderRadius: 6, border: '1px solid #1f2937', background: '#0b1320', color: '#e5e7eb' }}
        />
        <button disabled={busy || !text.trim()} style={{ padding: '8px 12px', borderRadius: 6, border: '1px solid #1f2937', background: '#1f2937', color: '#e5e7eb' }}>
          {busy ? 'Sending…' : 'Send'}
        </button>
      </form>
      <div style={{ marginTop: 8 }}>
        <BitsStatus bits={bits} />
      </div>
    </div>
  );
}
