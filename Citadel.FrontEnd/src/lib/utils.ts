import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
export function toFixedNumber(input: any, style?: keyof Intl.NumberFormatOptionsStyleRegistry, digits: number = 2) {
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
