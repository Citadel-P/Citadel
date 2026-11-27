import { useId, type ReactNode } from 'react';
import { Link } from 'react-router';
import { CitadelIcons } from '@/lib/icons';
import { Timestamp } from '@/components/custom/timestamp';
import type { DateTimeFormatter } from '@/lib/date-time';

const toneClasses = {
  success: 'border-success/30 text-success',
  warning: 'border-warning/30 text-warning',
  failure: 'border-destructive/30 text-destructive',
  info: 'border-info/30 text-info',
  neutral: 'border-border text-muted-foreground',
};

export function TimelineItem({
  format,
  type,
  title,
  onOpen,
  to,
  compact,
  time,
  status,
  statusLabel,
  actions,
  tone,
  context,
  resourceName,
}: {
  format: DateTimeFormatter;
  tone: keyof typeof toneClasses;
  type: string;
  title: string;
  onOpen?: () => void;
  to?: string;
  compact?: boolean;
  time?: string;
  status?: ReactNode;
  statusLabel?: string;
  actions?: ReactNode;
  context?: ReactNode;
  resourceName?: string;
}) {
  const statusId = useId();
  const eventLabel = resourceName ? `${title} · ${resourceName}` : title;
  const tooltip = statusLabel ? `${eventLabel} · ${statusLabel}` : eventLabel;
  const label = (
    <>
      {title}
      {resourceName && (
        <>
          <span className="mx-1.5 text-muted-foreground">·</span>
          <span className="dashboard-timeline-resource">{resourceName}</span>
        </>
      )}
    </>
  );
  const labelClass = 'dashboard-timeline-label min-w-0 truncate text-left text-sm font-medium';
  const linkClass = 'table-link after:absolute after:inset-0 ' + labelClass;
  const Icon = Object.hasOwn(CitadelIcons, type)
    ? CitadelIcons[type as keyof typeof CitadelIcons]
    : CitadelIcons.Activity;
  return (
    <li
      data-compact={compact || undefined}
      className="dashboard-timeline-item relative flex min-w-0 items-start gap-2 py-1 after:pointer-events-none after:absolute after:inset-s-2.5 after:top-3.5 after:-bottom-3.5 after:border-s after:border-border last:after:hidden">
      <span
        aria-hidden="true"
        className={
          'pointer-events-none relative z-1 flex size-5 shrink-0 items-center justify-center rounded-full border bg-card ' +
          toneClasses[tone]
        }>
        <Icon className="size-3" />
      </span>
      <div className="min-w-0 flex-1">
        <div className="dashboard-timeline-content flex min-h-5 min-w-0 items-center gap-2">
          <div className="flex min-w-0 flex-1 items-center gap-2">
            {to ? (
              <Link to={to} className={linkClass} aria-label={eventLabel} title={tooltip}>
                {label}
              </Link>
            ) : onOpen ? (
              <button
                type="button"
                className={linkClass}
                aria-label={resourceName ? eventLabel : undefined}
                aria-describedby={statusLabel ? statusId : undefined}
                title={tooltip}
                onClick={onOpen}>
                {label}
              </button>
            ) : (
              <span className={labelClass} title={tooltip}>
                {label}
              </span>
            )}
            {statusLabel && (
              <span className="sr-only" id={statusId}>
                {statusLabel}
              </span>
            )}
            {status && <span className="inline-flex shrink-0 items-center gap-1">{status}</span>}
          </div>
          <div className="dashboard-timeline-meta ms-auto flex shrink-0 items-center gap-1 text-xs text-muted-foreground">
            {time && <Timestamp className="text-xs text-muted-foreground" formatDateTime={format} value={time} />}
            {actions && <div className="relative">{actions}</div>}
          </div>
        </div>
        {context && (
          <div className="mt-0.5 w-fit max-w-full truncate text-xs text-muted-foreground">
            <span className="relative">{context}</span>
          </div>
        )}
      </div>
    </li>
  );
}
