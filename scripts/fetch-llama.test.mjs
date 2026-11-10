import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, lstatSync, mkdtempSync, readdirSync, readFileSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { keep, oneFilePerLibrary } from './fetch-llama.mjs';

test('GPU-1.1 the Vulkan backend is kept on both systems', () => {
  for (const name of ['ggml-vulkan.dll', 'libggml-vulkan.so']) assert.ok(keep(name), name);
});

test('LLM-3.1 the bundle keeps llama-server and its libraries, not the other llama.cpp tools', () => {
  for (const name of ['llama-server.exe', 'llama-server', 'llama-server-impl.dll', 'libllama-server-impl.so', 'llama.dll', 'llama-common.dll',
    'mtmd.dll', 'ggml-cpu-haswell.dll', 'libomp.dll', 'libggml-base.so.0', 'libllama.so.0.6.0', 'LICENSE', 'LICENSE-LLVM-OpenMP']) {
    assert.ok(keep(name), name);
  }
  for (const name of ['llama-cli.exe', 'llama-cli-impl.dll', 'libllama-bench-impl.so', 'llama-quantize', 'ggml-rpc-server', 'llama.exe']) {
    assert.ok(!keep(name), name);
  }
});

// Symlinks need extra rights on Windows; the Linux CI job runs it.
test('LLM-3.1 Linux libraries are bundled once, under their soname', { skip: process.platform === 'win32' }, () => {
  const d = mkdtempSync(join(tmpdir(), 'cursora-so-'));
  writeFileSync(join(d, 'libllama.so.0.6.0'), 'llama');
  symlinkSync('libllama.so.0.6.0', join(d, 'libllama.so.0'));
  symlinkSync('libllama.so.0', join(d, 'libllama.so'));
  writeFileSync(join(d, 'libggml-base.so.0.26.0'), 'base');
  symlinkSync('libggml-base.so.0.26.0', join(d, 'libggml-base.so.0'));
  symlinkSync('libggml-base.so.0', join(d, 'libggml-base.so'));
  writeFileSync(join(d, 'libggml-vulkan.so'), 'vulkan');
  oneFilePerLibrary(d);
  assert.deepEqual(readdirSync(d).sort(), ['libggml-base.so.0', 'libggml-vulkan.so', 'libllama.so.0']);
  assert.equal(readFileSync(join(d, 'libllama.so.0'), 'utf8'), 'llama');
  assert.ok(!lstatSync(join(d, 'libllama.so.0')).isSymbolicLink());
  assert.ok(existsSync(join(d, 'libggml-base.so.0')));
});
