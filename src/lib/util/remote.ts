/** Name rule from the backend: 1 to 40 letters, digits, dashes or underscores. */
const NAME_PATTERN = /^[A-Za-z0-9_-]{1,40}$/;

export function isValidRemoteName(name: string): boolean {
  return NAME_PATTERN.test(name.trim());
}
