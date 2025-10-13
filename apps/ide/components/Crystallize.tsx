import React from 'react';
import { crystallize } from '../lib/api';

export function Crystallize({ branchId, name, visible, onDone }: { branchId: string; name?: string; visible?: boolean; onDone: (r: any) => void }) {
  if (!visible) return null;
  async function onClick() {
    const res = await crystallize({ branchId, name });
    onDone(res);
  }
  return (
    <button onClick={onClick} style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #14532d', background: '#064e3b', color: '#d1fae5' }}>
      Crystallize
    </button>
  );
}
