import {describe,expect,it} from 'vitest';
import {stripDeclaredReadings} from '../src/services/strip-declared-readings';
import runtimeSrc from '../src/compat/runtime.js?raw';
describe('declared notices removed from the report page',()=>{
 it('drops the [Declared readings] block and keeps text before it',()=>{
  expect(stripDeclaredReadings('[Declared readings] The AFL as written.')).toBe('');
  expect(stripDeclaredReadings('[Stop in $] Not applied. [Declared readings] RSI(5) x [Not from the book] y')).toBe('[Stop in $] Not applied.');
  expect(stripDeclaredReadings('[Declared readings, not from the book] z')).toBe('');
  expect(stripDeclaredReadings('Plain note.')).toBe('Plain note.');
  expect(stripDeclaredReadings(undefined)).toBe('');
 });
 it('no longer emits the declared-application / rule-review boxes',()=>{
  expect(runtimeSrc).not.toContain('data-declared-application');
  expect(runtimeSrc).not.toContain('data-rule-review');
  expect(runtimeSrc).not.toContain('Earlier source-study comparison');
 });
});
