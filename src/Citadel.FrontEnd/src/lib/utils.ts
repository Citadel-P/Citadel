import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
import { truncate } from './truncate';
import { DockerResourceType, KnownResourceName } from '@/api/types';

/**
 * A utility function to conditionally join CSS class names together.
 * It uses `clsx` to handle conditional classes and `tailwind-merge` to resolve conflicting Tailwind CSS classes.
 *
 * @param inputs The class values to merge.
 * @returns The merged class name string.
 */
export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

/**
 * Formats a number to a fixed number of decimal places or as a percentage string.
 *
 * @param input The number to format.
 * @param style The formatting style to use. Currently, only 'percent' is handled to format as a percentage.
 * @param digits The number of digits to appear after the decimal point. Defaults to 2.
 * @returns The formatted number as a string, or undefined if the input is not provided.
 */
export function toFixedNumber(
  input: number | undefined | null,
  style?: 'percent',
  digits: number = 2,
): string | undefined {
  if (input === null || input === undefined) {
    return undefined;
  }
  if (input === 0) {
    return style === 'percent' ? '0%' : '0';
  }

  if (style === 'percent') {
    return new Intl.NumberFormat('default', {
      style: 'percent',
      minimumFractionDigits: digits,
      maximumFractionDigits: digits,
    }).format(input / 100);
  }

  return input.toFixed(digits);
}

/**
 * Recursively extracts the edited fields from a form state, based on the `dirtyFields` object.
 * This is useful for sending only the changed data to the server.
 *
 * @param dirtyFields An object representing the dirty fields in the form. Can be a boolean for nested objects.
 * @param allValues An object containing all the current values of the form.
 * @returns An object containing only the fields that have been edited.
 */
export function getEditedFields(dirtyFields: object | boolean, allValues: object): object {
  // If dirtyFields is true, it means the entire object is dirty, so return all values.
  if (dirtyFields === true || Array.isArray(dirtyFields)) {
    return allValues;
  }

  // Recursively process each key in the dirtyFields object.
  return Object.fromEntries(
    Object.keys(dirtyFields).map((key) => [
      key,
      getEditedFields(dirtyFields[key as keyof typeof dirtyFields], allValues[key as keyof typeof allValues]),
    ]),
  );
}

/**
 * Formats a large number into a human-readable string with a suffix (k, M, B).
 *
 * @param value The number to format.
 * @returns The formatted number string.
 */
export function formatNumber(value: number): string {
  if (value >= 1_000_000_000) {
    return `${(value / 1_000_000_000).toFixed(1)}B`;
  }
  if (value >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(1)}M`;
  }
  if (value >= 1_000) {
    return `${(value / 1_000).toFixed(1)}k`;
  }
  return value.toString();
}

export function formatId(id?: string): string {
  if (!id) return '';

  const normalized = id.startsWith('sha256:') ? id.slice(7) : id;

  return truncate(normalized, 12, 'right', true);
}

export const filterBySplit = <T>(items: T[] | undefined, search: string, extract: (item: T) => string) => {
  const split = search.toLowerCase().split(' ');
  return (
    (split.length
      ? items?.filter((item) => {
          const target = extract(item).toLowerCase();
          return split.every((term) => target.includes(term));
        })
      : items) ?? []
  );
};

export const pluralize = (word: string) => (word.endsWith('y') ? word.slice(0, -1) + 'ies' : word + 's');
