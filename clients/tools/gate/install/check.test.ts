import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

// Verifies: SEC-SUP-033. CLI tests own exit status and JSON wiring only.
test('policy CLI returns successful JSON for a valid collected manifest', () => {
  const result = spawnSync(process.execPath, [fileURLToPath(new URL('check.ts', import.meta.url)), 'manifest'], {
    input: JSON.stringify({ manifest: {}, members: [] }), encoding: 'utf8',
  });
  assert.deepEqual({ status: result.status, signal: result.signal, stdout: result.stdout, stderr: result.stderr }, {
    status: 0, signal: null, stdout: '[]\n', stderr: '',
  });
});

test('policy CLI preserves a named refusal and nonzero status', () => {
  const result = spawnSync(process.execPath, [fileURLToPath(new URL('check.ts', import.meta.url)), 'unknown'], {
    input: '{}', encoding: 'utf8',
  });
  assert.deepEqual({ status: result.status, signal: result.signal, stdout: result.stdout, stderr: result.stderr }, {
    status: 1, signal: null,
    stdout: '[{"rule":"SEC-SUP-033","path":"input","message":"unknown policy check"}]\n', stderr: '',
  });
});
