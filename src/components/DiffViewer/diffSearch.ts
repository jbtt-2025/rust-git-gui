/**
 * Pure text search utility for DiffViewer.
 * Returns all starting indices of case-insensitive matches.
 */
export function findAllMatches(text: string, query: string): number[] {
  if (!query || !text) return [];

  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();
  const results: number[] = [];
  let pos = 0;

  while (pos <= lowerText.length - lowerQuery.length) {
    const idx = lowerText.indexOf(lowerQuery, pos);
    if (idx === -1) break;
    results.push(idx);
    pos = idx + 1;
  }

  return results;
}
