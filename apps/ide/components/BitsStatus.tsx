import React from 'react';

type Bits = {
  A?: number; // Aligned
  U?: number; // Uncertain / Needs Evidence
  P?: number; // Pending Approval
  E?: number; // Error/Evidence?
  [k: string]: any;
};

export function BitsStatus({ bits }: { bits?: Bits }) {
  if (!bits) return null;

  const A = bits.A ?? 0;
  const U = bits.U ?? 0;
  const P = bits.P ?? 0;
  const E = bits.E ?? 0;

  let status = 'Idle';
  if (E) status = 'Error';
  else if (P) status = 'Aligned; Awaiting Approval';
  else if (U) status = 'Needs Evidence';
  else if (A) status = 'Aligned';

  return (
    <div className="status">
      <strong>Status:</strong> {status}
      <span className="muted" style={{ marginLeft: 8 }}>
        (A:{A} U:{U} P:{P} E:{E})
      </span>
    </div>
  );
}
