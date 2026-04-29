/**
 * Validate the clone form fields.
 * Returns an object with i18n error keys for invalid fields.
 * An empty object means the form is valid.
 */
export function validateCloneForm(
  url: string,
  path: string
): { url?: string; path?: string } {
  const errors: { url?: string; path?: string } = {};

  if (!url.trim()) {
    errors.url = 'welcome.urlRequired';
  }

  if (!path.trim()) {
    errors.path = 'welcome.pathRequired';
  }

  return errors;
}
