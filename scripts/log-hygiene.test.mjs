import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

// Names that hold user content or secrets must never reach a log macro (APP-7.2).
const FORBIDDEN = /\b(text|selection|content|question|answer|prompt|messages?|api_?key|secret|password|token)\b/i;

// String literals are reduced to their `{name}` captures, so wording in messages is fine but `{selection}` is not.
function offending(source) {
  const found = [];
  for (const m of source.matchAll(/\b(?:trace|debug|info|warn|error)!\(([\s\S]*?)\);/g)) {
    const args = m[1].replace(/"(?:[^"\\]|\\.)*"/g, (s) => (s.match(/\{\w+/g) ?? []).join(' '));
    if (FORBIDDEN.test(args)) found.push(m[0].slice(0, 100));
  }
  return found;
}

function* rustFiles(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) yield* rustFiles(p);
    else if (p.endsWith('.rs')) yield p;
  }
}

test('APP-7.2 Rust log calls carry no user content or secrets', () => {
  assert.equal(offending('log::info!("captured {selection}");').length, 1);
  assert.deepEqual(offending('log::info!("selection has {} chars", n);'), []);

  const found = [];
  for (const file of rustFiles('src-tauri/src')) for (const o of offending(fs.readFileSync(file, 'utf8'))) found.push(`${file}: ${o}`);
  assert.deepEqual(found, []);
});
