import './globals.css';
import React, { useEffect, useMemo, useState } from 'react';
import { AutodocPanel } from '../components/AutodocPanel';
import { Chat } from '../components/Chat';
import { ENGINE_BASE_URL, getAutodoc } from '../lib/api';
import { Crystallize } from '../components/Crystallize';

function useBranchId(): [string, (v: string) => void] {
  const [branchId, setBranchId] = useState<string>(() => {
    if (typeof window === 'undefined') return 'dev';
    return localStorage.getItem('branch_id') || 'dev';
  });
  useEffect(() => {
    if (typeof window !== 'undefined') localStorage.setItem('branch_id', branchId);
  }, [branchId]);
  return [branchId, setBranchId];
}

export default function Page() {
  const [branchId, setBranchId] = useBranchId();
  const [last, setLast] = useState<any | null>(null);
  const [auto, setAuto] = useState<any | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function run() {
      try {
        const a = await getAutodoc(branchId);
        if (!cancelled) setAuto(a);
      } catch (e) {
        // ignore; panel renders names separately
      }
    }
    run();
    const t = setInterval(run, 6000);
    return () => { cancelled = true; clearInterval(t); };
  }, [branchId]);

  const crystVisible = useMemo(() => Boolean(last?.is_crystallizable), [last]);

  return (
    <>
      <header>
        <div className="row" style={{ justifyContent: 'space-between' }}>
          <div>
            <strong>Consciousness IDE</strong>
            <span className="muted" style={{ marginLeft: 10 }}>Engine: {ENGINE_BASE_URL}</span>
          </div>
          <div className="row">
            <label htmlFor="branch" className="muted">Branch:</label>
            <input id="branch" value={branchId} onChange={(e) => setBranchId(e.target.value)} style={{ padding: 6, borderRadius: 6, border: '1px solid #1f2937', background: '#0b1320', color: '#e5e7eb' }} />
          </div>
        </div>
      </header>
      <main>
        <div className="grid">
          <AutodocPanel branchId={branchId} />
          <div className="card">
            <strong>Chat</strong>
            <div style={{ marginTop: 8 }}>
              <Chat branchId={branchId} onResult={setLast} />
            </div>
            <div className="row" style={{ justifyContent: 'space-between', marginTop: 8 }}>
              <div className="muted">is_crystallizable: {String(Boolean(last?.is_crystallizable))}</div>
              <Crystallize branchId={branchId} name={last?.name} visible={crystVisible} onDone={setLast} />
            </div>
          </div>
        </div>
        <div className="card" style={{ marginTop: 12 }}>
          <strong>Last Response</strong>
          <pre style={{ whiteSpace: 'pre-wrap', overflowX: 'auto' }}>{JSON.stringify(last, null, 2)}</pre>
        </div>
      </main>
    </>
  );
}
