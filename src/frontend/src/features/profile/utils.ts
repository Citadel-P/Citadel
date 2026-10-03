import { UserDateTimeFormat } from '@/api/generated/api.types';
import { formatDateTime } from '@/lib/date-time';

export type ProfileDateFormatter = (value: unknown) => string;

export function getInitials(value: string | null | undefined) {
  const parts = (value ?? '')
    .trim()
    .split(/\s+/)
    .filter(Boolean);

  if (parts.length === 0) return '?';
  return parts
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase())
    .join('');
}

export function formatProfileDateTime(
  value: unknown,
  timeZone: string | null | undefined,
  format: UserDateTimeFormat | undefined,
) {
  return formatDateTime(value, timeZone, format);
}
