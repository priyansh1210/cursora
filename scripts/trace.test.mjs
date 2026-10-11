import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { trace } from './trace.mjs';

// Fixture prefixes (ZZZ, ZZY) are not real spec prefixes, so the real-repo trace ignores them.
function fixture(files) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'trace-'));
  for (const [p, text] of Object.entries(files)) {
    fs.mkdirSync(path.dirname(path.join(root, p)), { recursive: true });
    fs.writeFileSync(path.join(root, p), text);
  }
  return root;
}

const specs = {
  'specs/README.md':
    '| 01 | [A](01-a/requirements.md) | ZZZ | Approved | Approved | Approved | In progress |\r\n' +
    '| 02 | [B](02-b/requirements.md) | ZZY | Approved | — | — | Approved (reqs) |\r\n',
  'specs/01-a/requirements.md': '- ZZZ-1.1 (auto) a\n- ZZZ-1.2 (auto) b\n- ZZZ-1.3 (manual) c\n- ZZZ-1.4 (retired) d\n',
  'specs/02-b/requirements.md': '- ZZY-1.1 (auto) later\n',
};

test('APP-8.1 passes when every auto criterion of a started spec has a test', () => {
  const r = trace(fixture({
    ...specs,
    'src-tauri/src/a.rs': '#[test]\nfn zzz_1_1_works() {}\n',
    'src/a.test.ts': "it('ZZZ-1.2 works', () => {});\n",
  }));
  assert.deepEqual(r.missing, []);
  assert.deepEqual(r.unknown, []);
  assert.equal(r.pending, 1); // ZZY-1.1 is in a spec that has not started
});

test('APP-8.1 fails on an uncovered auto criterion and on unknown or retired IDs', () => {
  const r = trace(fixture({ ...specs, 'src/a.test.ts': "it('ZZZ-1.2'); it('ZZZ-9.9'); it('ZZZ-1.4');\n" }));
  assert.deepEqual(r.missing, ['ZZZ-1.1']);
  assert.deepEqual(r.unknown.map(([id]) => id), ['ZZZ-9.9', 'ZZZ-1.4']);
});
