/**
 * Pure function to check commit title (first line) length.
 * Returns the character count and whether it exceeds the 72-char limit.
 */
export function checkCommitTitle(title: string): { length: number; warning: boolean } {
  const length = title.length;
  return { length, warning: length > 72 };
}
