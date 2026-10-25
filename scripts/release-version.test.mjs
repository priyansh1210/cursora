import test from 'node:test';
import assert from 'node:assert/strict';
import { tagMatches } from './release-version.mjs';

test("UPD-3.4 a release tag must be v plus the app's version, with an optional pre-release suffix", () => {
  for (const tag of ['v0.2.0', 'v0.2.0-rc.1']) assert.ok(tagMatches(tag, '0.2.0'), tag);
  for (const tag of ['v0.1.9', '0.2.0', 'v0.2.0.1', 'v0.2.0-', 'v0.2.00', '']) assert.ok(!tagMatches(tag, '0.2.0'), tag);
});
