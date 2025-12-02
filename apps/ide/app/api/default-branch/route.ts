import { NextResponse } from 'next/server';
import { promises as fs } from 'fs';
import path from 'path';

export async function GET() {
  try {
    // apps/ide/app/api/default-branch/route.ts -> monorepo root is two levels up
    const root = path.resolve(process.cwd(), '..', '..');
    const p = path.join(root, 'out_one_engine', 'branch_id.txt');
    const raw = await fs.readFile(p, 'utf8');
    const branchId = raw.trim();
    if (!branchId) return NextResponse.json({ ok: false }, { status: 404 });
    return NextResponse.json({ ok: true, branchId });
  } catch (e) {
    return NextResponse.json({ ok: false, error: String(e) }, { status: 404 });
  }
}
