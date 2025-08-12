const trailingChars = '...';

/**
 * Truncates a string to a specified length, with options for direction and a trailing ellipsis.
 *
 * @param value The string to truncate.
 * @param limit The maximum length of the truncated string. Defaults to 18.
 * @param dir The direction from which to truncate the string ('left' or 'right'). Defaults to 'right'.
 * @param noTrail If true, the trailing ellipsis will not be added. Defaults to false.
 * @returns The truncated string.
 */
export function truncate(value: string, limit: number = 18, dir: 'left' | 'right' = 'right', noTrail: boolean = false): string {
  if (value.length <= limit) {
    return value;
  }

  const trail = noTrail ? '' : trailingChars;

  if (dir === 'left') {
    // Truncate from the beginning of the string.
    return trail + value.substring(value.length - limit);
  } else {
    // Truncate from the end of the string.
    return value.substring(0, limit) + trail;
  }
}