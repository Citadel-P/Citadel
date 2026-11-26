import { Link } from 'react-router';
import type { ReactNode } from 'react';
import { CitadelIcons } from '@/lib/icons';
import { Timestamp } from '@/components/custom/timestamp';
import type { DateTimeFormatter } from '@/lib/date-time';

export function TimelineItem({
  format,
  type,
  title,
  path,
  time,
  status,
  actions,
  failed = false,
  context,
}: {
  format: DateTimeFormatter;
  failed?: boolean;
  type: string;
  title: string;
  path: string;
  time: string;
  status: ReactNode;
  actions: ReactNode;
  context?: ReactNode;
}) {
  const Icon = Object.hasOwn(CitadelIcons, type)
    ? CitadelIcons[type as keyof typeof CitadelIcons]
    : CitadelIcons.Activity;
  return (
    <li className="group relative flex min-w-0 items-start gap-2.5 py-1.5 after:pointer-events-none after:absolute after:inset-s-3 after:top-[2rem] after:bottom-[-0.375rem] after:border-s after:border-border last:after:hidden">
      <span
        className={
          'relative z-1 flex size-6 shrink-0 items-center justify-center rounded-full border bg-card ' +
          (failed ? 'border-destructive/40 text-destructive' : 'border-blue-500/45 text-blue-600 dark:text-blue-400')
        }>
        <Icon className="size-3.5" />
      </span>
      <div className="min-w-0 flex-1 rounded-md border bg-muted/20 px-3 py-2">
        <div className="flex flex-wrap items-baseline justify-between gap-x-2">
          <Link className="table-link min-w-0 truncate text-sm font-medium" title={title} to={path}>
            {title}
          </Link>
          <Timestamp className="shrink-0 text-xs text-muted-foreground" formatDateTime={format} value={time} />
        </div>
        <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
          {status}
          {context}
          <div className="ms-auto">{actions}</div>
        </div>
      </div>
    </li>
  );
}
