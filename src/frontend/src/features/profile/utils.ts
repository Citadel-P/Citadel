import type { DateTimeFormatter } from '@/lib/date-time';

export type ProfileDateFormatter = DateTimeFormatter;

export function getInitials(value: string | null | undefined) {
  const parts = (value ?? '').trim().split(/\s+/).filter(Boolean);

  if (parts.length === 0) return '?';
  return parts
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase())
    .join('');
}
