import { UserDateTimeFormat, UserDateDisplay } from '@/api/generated/api.types';
import { DateTimeFormatter, formatDateTime, getBrowserTimezone } from '@/lib/date-time';
import { fromNow } from '@/lib/dayjs.helper';
import { useRead } from '@/lib/hooks';
import { useMemo } from 'react';

export function useProfileDateTimeFormatter(): DateTimeFormatter {
  const preferencesQuery = useRead('getProfilePreferences', undefined, { staleTime: Infinity });
  const preferences = preferencesQuery.data?.data;

  return useMemo(
    () => (value, mode) => {
      if (mode !== 'exact' && preferences?.dateDisplay !== UserDateDisplay.Exact) return fromNow(value);
      return formatDateTime(
        value,
        preferences?.timeZone ?? getBrowserTimezone(),
        preferences?.dateTimeFormat ?? UserDateTimeFormat.System,
      );
    },
    [preferences?.dateTimeFormat, preferences?.timeZone, preferences?.dateDisplay],
  );
}
