import test from 'node:test';
import assert from 'node:assert/strict';
import { keep } from './fetch-llama.mjs';

test('LLM-3.1 the bundle keeps llama-server and its libraries, not the other llama.cpp tools', () => {
  for (const name of ['llama-server.exe', 'llama-server', 'llama-server-impl.dll', 'libllama-server-impl.so', 'llama.dll', 'llama-common.dll',
    'mtmd.dll', 'ggml-cpu-haswell.dll', 'libomp.dll', 'libggml-base.so.0', 'libllama.so.0.6.0', 'LICENSE', 'LICENSE-LLVM-OpenMP']) {
    assert.ok(keep(name), name);
  }
  for (const name of ['llama-cli.exe', 'llama-cli-impl.dll', 'libllama-bench-impl.so', 'llama-quantize', 'ggml-rpc-server', 'llama.exe']) {
    assert.ok(!keep(name), name);
  }
});
