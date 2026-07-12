import { UserDateTimeFormat } from '@/api/generated/api.types';
import { DateTimeFormatter, formatDateTime, getBrowserTimezone } from '@/lib/date-time';
import { useRead } from '@/lib/hooks';
import { useMemo } from 'react';

export function useProfileDateTimeFormatter(): DateTimeFormatter {
  const preferencesQuery = useRead('getProfilePreferences');
  const preferences = preferencesQuery.data?.data;

  return useMemo(
    () => (value: unknown) =>
      formatDateTime(
        value,
        preferences?.timeZone ?? getBrowserTimezone(),
        preferences?.dateTimeFormat ?? UserDateTimeFormat.System,
      ),
    [preferences?.dateTimeFormat, preferences?.timeZone],
  );
}
