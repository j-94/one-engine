export const ENGINE_BASE_URL = (process.env.NEXT_PUBLIC_ENGINE_BASE_URL || 'http://127.0.0.1:8000').replace(/\/$/, '');

export async function getAutodoc(branchId: string) {
  const res = await fetch(`${ENGINE_BASE_URL}/autodoc/${encodeURIComponent(branchId)}`, { cache: 'no-store' });
  if (!res.ok) throw new Error(`autodoc ${res.status}`);
  return res.json();
}

export async function getAutodocNames(branchId: string) {
  const res = await fetch(`${ENGINE_BASE_URL}/autodoc/${encodeURIComponent(branchId)}/names`, { cache: 'no-store' });
  if (!res.ok) return [] as string[];
  return res.json();
}

export async function sendPrompt(branchId: string, prompt: string) {
  const res = await fetch(`${ENGINE_BASE_URL}/conversation/${encodeURIComponent(branchId)}/prompt`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ prompt })
  });
  if (!res.ok) throw new Error(`prompt ${res.status}`);
  return res.json();
}

export async function crystallize(payload: { branchId: string; name?: string }) {
  const res = await fetch(`${ENGINE_BASE_URL}/crystallize`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload)
  });
  return res.ok ? res.json() : { ok: false, status: res.status };
}
