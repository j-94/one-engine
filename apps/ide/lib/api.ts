export const ENGINE_BASE_URL = (process.env.NEXT_PUBLIC_ENGINE_BASE_URL || 'http://127.0.0.1:7777').replace(/\/$/, '');

export async function getAutodoc(branchId: string) {
  const res = await fetch(`${ENGINE_BASE_URL}/autodoc/${encodeURIComponent(branchId)}`, { cache: 'no-store' });
  if (!res.ok) throw new Error(`autodoc ${res.status}`);
  return res.json();
}

export async function getAutodocNames(branchId: string) {
  // Some engine versions expose /autodoc/{branchId} with a shape { branch_id, label, endpoints: [...] }
  // Fallback: if the API returns an array, assume it's already a list of names.
  const res = await fetch(`${ENGINE_BASE_URL}/autodoc/${encodeURIComponent(branchId)}`, { cache: 'no-store' });
  if (!res.ok) return [] as string[];
  const data = await res.json();
  if (Array.isArray(data)) return data as string[];
  if (data && Array.isArray(data.endpoints)) {
    return (data.endpoints as any[])
      .map((e: any) => (typeof e === 'string' ? e : (e?.name || e?.title || e?.api_name || '')))
      .filter(Boolean);
  }
  return [] as string[];
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
