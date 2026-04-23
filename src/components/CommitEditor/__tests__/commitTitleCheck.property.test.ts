/**
 * Property 11: 提交标题长度警告
 *
 * For any string as a commit message title line, the character count function
 * SHALL return an accurate character count, and the warning flag SHALL be
 * triggered if and only if the character count exceeds 72.
 *
 * **Validates: Requirements 5.5**
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import { checkCommitTitle } from '../commitTitleCheck';

describe('Property 11: Commit title length warning', () => {
  it('length equals the actual string length for any string', () => {
    fc.assert(
      fc.property(fc.string(), (title) => {
        const result = checkCommitTitle(title);
        expect(result.length).toBe(title.length);
      }),
    );
  });

  it('warning is true if and only if length > 72', () => {
    fc.assert(
      fc.property(fc.string(), (title) => {
        const result = checkCommitTitle(title);
        expect(result.warning).toBe(result.length > 72);
      }),
    );
  });

  it('a string of exactly 72 characters does NOT trigger a warning', () => {
    fc.assert(
      fc.property(fc.char(), (ch) => {
        const title = ch.repeat(72);
        const result = checkCommitTitle(title);
        expect(result.length).toBe(72);
        expect(result.warning).toBe(false);
      }),
    );
  });

  it('a string of exactly 73 characters SHOULD trigger a warning', () => {
    fc.assert(
      fc.property(fc.char(), (ch) => {
        const title = ch.repeat(73);
        const result = checkCommitTitle(title);
        expect(result.length).toBe(73);
        expect(result.warning).toBe(true);
      }),
    );
  });

  it('empty string returns length 0 and no warning', () => {
    const result = checkCommitTitle('');
    expect(result.length).toBe(0);
    expect(result.warning).toBe(false);
  });
});
