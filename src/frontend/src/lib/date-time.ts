import { UserDateTimeFormat } from '@/api/generated/api.types';

export type DateTimeFormatter = (value: unknown) => string;

const ISO_WITHOUT_OFFSET = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?$/;

export function getBrowserTimezone() {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
}

export function parseCitadelDate(value: unknown): Date | null {
  if (!value) return null;
  if (value instanceof Date) return Number.isNaN(value.getTime()) ? null : value;

  const dateValue =
    typeof value === 'string' && ISO_WITHOUT_OFFSET.test(value)
      ? new Date(`${value}Z`)
      : new Date(value as string | number);

  return Number.isNaN(dateValue.getTime()) ? null : dateValue;
}

export function formatDateTime(
  value: unknown,
  timeZone: string | null | undefined,
  format: UserDateTimeFormat | undefined,
) {
  const date = parseCitadelDate(value);
  if (!date) return '-';

  const hour12 =
    format === UserDateTimeFormat.TwelveHour ? true : format === UserDateTimeFormat.TwentyFourHour ? false : undefined;

  const options: Intl.DateTimeFormatOptions = {
    dateStyle: 'medium',
    timeStyle: 'short',
    timeZone: timeZone || getBrowserTimezone(),
    hour12,
  };

  try {
    return new Intl.DateTimeFormat(undefined, options).format(date);
  } catch {
    return new Intl.DateTimeFormat(undefined, { ...options, timeZone: 'UTC' }).format(date);
  }
}
