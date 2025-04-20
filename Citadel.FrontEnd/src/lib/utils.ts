import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function toFixedNumber(input: any, style?: keyof Intl.NumberFormatOptionsStyleRegistry, digits: number = 2) {
  if (input === 0) {
    return '0%';
  }
  if (!input) {
    return;
  }
  if (style) {
    return Intl.NumberFormat('default', {
      style,
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(input / 100);
  }
  return Number.parseFloat(input).toFixed(digits);
}

export function getEditedFields(dirtyFields: object | boolean, allValues: object): object {
  if (dirtyFields === true || Array.isArray(dirtyFields)) return allValues;
  return Object.fromEntries(
    Object.keys(dirtyFields).map((key) => [key, getEditedFields(dirtyFields[key], allValues[key])]),
  );
}

export function formatNumber(value: number): string {
  if (value >= 1_000_000_000) {
    return `${(value / 1_000_000_000).toFixed(1)}B`;
  } else if (value >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(1)}M`;
  } else if (value >= 1_000) {
    return `${(value / 1_000).toFixed(1)}k`;
  }
  return value.toString();
}
