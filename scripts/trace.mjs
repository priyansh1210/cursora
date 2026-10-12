// Traceability check (constitution §2): every (auto) criterion of a started spec has a test,
// and tests cite only IDs that exist. Usage: node scripts/trace.mjs [repoRoot]
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

export function trace(root) {
  const specsDir = path.join(root, 'specs');

  // Coverage is enforced only for specs whose README status is "In progress" or "Done".
  const started = new Set();
  const readme = fs.readFileSync(path.join(specsDir, 'README.md'), 'utf8');
  for (const m of readme.matchAll(/^\|\s*\d+\s*\|\s*\[[^\]]*\]\(([^/]+)\/requirements\.md\).*\|\s*([^|]+?)\s*\|\s*$/gm))
    if (/^(In progress|Done)/.test(m[2])) started.add(m[1]);

  const criteria = new Map(); // id -> { tag, spec }
  for (const dir of fs.readdirSync(specsDir)) {
    const file = path.join(specsDir, dir, 'requirements.md');
    if (!fs.existsSync(file)) continue;
    for (const m of fs.readFileSync(file, 'utf8').matchAll(/^- ([A-Z]{3}-\d+\.\d+) \((auto|manual|retired)\)/gm))
      criteria.set(m[1], { tag: m[2], spec: dir });
  }

  // shortcut: only known prefixes count as IDs, so a typo'd prefix (SLE-1.1) goes unnoticed; fine until it bites.
  const prefixes = new Set([...criteria.keys()].map((id) => id.slice(0, 3)));
  const cited = new Map(); // id -> first test file citing it
  for (const file of testFiles(root)) {
    const text = fs.readFileSync(file, 'utf8');
    const ids = file.endsWith('.rs')
      ? [...text.matchAll(/\bfn\s+([a-z]{3})_(\d+)_(\d+)(?!\d)/g)].map((m) => `${m[1].toUpperCase()}-${m[2]}.${m[3]}`)
      : [...text.matchAll(/\b([A-Z]{3}-\d+\.\d+)\b/g)].map((m) => m[1]);
    for (const id of ids)
      if (prefixes.has(id.slice(0, 3)) && !cited.has(id)) cited.set(id, path.relative(root, file));
  }

  const auto = [...criteria].filter(([, c]) => c.tag === 'auto');
  return {
    missing: auto.filter(([id, c]) => started.has(c.spec) && !cited.has(id)).map(([id]) => id),
    unknown: [...cited].filter(([id]) => criteria.get(id)?.tag === undefined || criteria.get(id).tag === 'retired'),
    pending: auto.filter(([id, c]) => !started.has(c.spec) && !cited.has(id)).length,
    covered: auto.filter(([id]) => cited.has(id)).length,
    total: auto.length,
  };
}

function* testFiles(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (['node_modules', 'target', '.git', 'dist', 'specs', 'gen'].includes(e.name)) continue;
    const p = path.join(dir, e.name);
    if (e.isDirectory()) yield* testFiles(p);
    else if (p.endsWith('.rs') || /\.test\.(ts|tsx|js|mjs)$/.test(p)) yield p;
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const r = trace(process.argv[2] ?? process.cwd());
  for (const id of r.missing) console.error(`missing test: ${id}`);
  for (const [id, file] of r.unknown) console.error(`unknown or retired ID ${id} cited in ${file}`);
  console.log(`trace: ${r.covered}/${r.total} auto criteria covered, ${r.pending} pending in specs not started yet`);
  process.exitCode = r.missing.length || r.unknown.length ? 1 : 0;
}
