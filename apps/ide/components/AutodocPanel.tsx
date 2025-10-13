import React, { useEffect, useState } from 'react';
import { getAutodocNames } from '../lib/api';

export function AutodocPanel({ branchId }: { branchId: string }) {
  const [names, setNames] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function run() {
      setLoading(true);
      setError(null);
      try {
        const n = await getAutodocNames(branchId);
        if (!cancelled) setNames(n || []);
      } catch (e: any) {
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    run();
    const t = setInterval(run, 4000);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, [branchId]);

  return (
    <div className="card">
      <div className="row" style={{ justifyContent: 'space-between' }}>
        <strong>Pattern Library</strong>
        {loading && <span className="muted">Loading…</span>}
      </div>
      {error && <div style={{ color: '#fca5a5' }}>{error}</div>}
      {!error && (
        <div className="row" style={{ marginTop: 6, flexWrap: 'wrap' }}>
          {names?.length ? names.map(n => <span key={n} className="tag">{n}</span>) : <span className="muted">No persisted patterns</span>}
        </div>
      )}
    </div>
  );
}
