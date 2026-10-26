import { type DateTimeFormatter, parseCitadelDate } from '@/lib/date-time';
import { cn } from '@/lib/utils';

export function Timestamp({
  value,
  formatDateTime,
  className,
}: {
  value: unknown;
  formatDateTime: DateTimeFormatter;
  className?: string;
}) {
  const absolute = formatDateTime(value, 'exact');
  const displayed = formatDateTime(value);
  const date = parseCitadelDate(value);

  if (!date) return <span className={cn('text-[13px] text-muted-foreground', className)}>-</span>;

  return (
    <time
      dateTime={date.toISOString()}
      title={absolute}
      aria-label={absolute}
      className={cn('inline-block max-w-44 truncate align-bottom text-left text-[13px] text-foreground', className)}>
      {displayed}
    </time>
  );
}
