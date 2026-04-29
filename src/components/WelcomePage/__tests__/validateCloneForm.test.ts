import { describe, it, expect } from 'vitest';
import { validateCloneForm } from '../validateCloneForm';

describe('validateCloneForm', () => {
  it('returns no errors when both url and path are valid', () => {
    expect(validateCloneForm('https://github.com/user/repo.git', '/home/user/repo')).toEqual({});
  });

  it('returns url error when url is empty', () => {
    const errors = validateCloneForm('', '/home/user/repo');
    expect(errors.url).toBe('welcome.urlRequired');
    expect(errors.path).toBeUndefined();
  });

  it('returns url error when url is whitespace-only', () => {
    const errors = validateCloneForm('   ', '/home/user/repo');
    expect(errors.url).toBe('welcome.urlRequired');
    expect(errors.path).toBeUndefined();
  });

  it('returns path error when path is empty', () => {
    const errors = validateCloneForm('https://github.com/user/repo.git', '');
    expect(errors.path).toBe('welcome.pathRequired');
    expect(errors.url).toBeUndefined();
  });

  it('returns path error when path is whitespace-only', () => {
    const errors = validateCloneForm('https://github.com/user/repo.git', '  \t\n  ');
    expect(errors.path).toBe('welcome.pathRequired');
    expect(errors.url).toBeUndefined();
  });

  it('returns both errors when both url and path are empty', () => {
    const errors = validateCloneForm('', '');
    expect(errors.url).toBe('welcome.urlRequired');
    expect(errors.path).toBe('welcome.pathRequired');
  });

  it('returns both errors when both are whitespace-only', () => {
    const errors = validateCloneForm('   ', '   ');
    expect(errors.url).toBe('welcome.urlRequired');
    expect(errors.path).toBe('welcome.pathRequired');
  });
});
