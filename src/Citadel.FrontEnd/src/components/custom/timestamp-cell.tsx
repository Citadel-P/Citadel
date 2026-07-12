import type { DateTimeFormatter } from '@/lib/date-time';
import { fromNow } from '@/lib/dayjs.helper';

export function TimestampCell({
  value,
  formatDateTime,
}: {
  value: unknown;
  formatDateTime: DateTimeFormatter;
}) {
  const absolute = formatDateTime(value);
  const relative = fromNow(value);

  return (
    <span className="block max-w-44 truncate text-[13px] text-foreground" title={relative}>
      {absolute}
    </span>
  );
}
