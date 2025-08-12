import dayjs from 'dayjs';
import relativeTime from 'dayjs/plugin/relativeTime';

// Extend dayjs with the relativeTime plugin to support `fromNow` functionality.
dayjs.extend(relativeTime);

/**
 * Formats a date or timestamp into a human-readable relative time string (e.g., "a few seconds ago", "in 2 hours").
 *
 * @param {Date | number} value The date or timestamp to format.
 * @returns {string} The formatted relative time string.
 */
export const fromNow = (value: Date | number): string => dayjs(value).fromNow();
