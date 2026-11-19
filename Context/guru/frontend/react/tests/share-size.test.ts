import {expect, it} from 'vitest';
import {fmtBytes, fmtBytesBoth} from '../src/services/research';

// 16 Nov 2026: the share total was 2.14 TiB shown as "TB"; the NAS reports decimal TB.
it('binary sizes carry binary labels', () => {
  expect(fmtBytes(2_350_520_133_106)).toBe('2.14 TiB');
  expect(fmtBytes(1024 ** 3)).toBe('1 GiB');
  expect(fmtBytes(512)).toBe('512 bytes');
});
it('the share total shows decimal TB with the TiB in brackets', () => {
  expect(fmtBytesBoth(2_350_520_133_106)).toBe('2.35 TB (2.14 TiB)');
  expect(fmtBytesBoth(19_933_458_620_416)).toBe('19.9 TB (18.1 TiB)');
  expect(fmtBytesBoth(999)).toBe('999 bytes');
});
