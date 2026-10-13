import { BackupCoverageStatus as Status, ResourceBackupSummaryView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Popover, PopoverAnchor, PopoverContent } from '@/components/ui/popover';
import { cn } from '@/lib/utils';
import { Shield, ShieldAlert, ShieldCheck, ShieldOff } from 'lucide-react';
import { useEffect, useId, useRef, useState } from 'react';
import { backupSummaryStatus, backupConfigurationIssue } from './coverage';
import { ResourceBackupSummary, ResourceBackupSource } from './resource-backup-summary';

function coverageAppearance(coverage?: ResourceBackupSummaryView | null) {
  const neutral = { Icon: Shield, className: 'text-muted-foreground' };
  const aggregate = backupSummaryStatus(coverage);
  if (!coverage || aggregate === 'Unavailable') return { ...neutral, label: 'Coverage unavailable' };
  const hasBindMounts = coverage.volumes.some((volume) => volume.hostPath);
  const statuses = coverage.volumes.map((volume) => volume.status);
  if (aggregate === Status.NotApplicable) {
    return { ...neutral, label: hasBindMounts ? 'No eligible storage targets' : 'No eligible volumes' };
  }
  if (aggregate === Status.Failed) {
    return { Icon: ShieldAlert, className: 'text-destructive', label: 'Backup failed' };
  }
  if (
    aggregate === Status.Warning &&
    (!statuses.includes(Status.Unprotected) ||
      statuses.includes(Status.Warning) ||
      coverage.warnings.length > 0 ||
      (coverage.reasons?.length ?? 0) > 0)
  ) {
    return {
      Icon: ShieldAlert,
      className: 'text-amber-700 dark:text-amber-400',
      label: coverage.reasons?.some((reason) => reason === 'SchedulingUnavailable')
        ? 'Automatic backups paused'
        : (backupConfigurationIssue(coverage.policies) ?? 'Needs attention'),
    };
  }
  if (statuses.includes(Status.Unprotected)) {
    return {
      Icon: ShieldOff,
      className: 'text-amber-700 dark:text-amber-400',
      label: statuses.includes(Status.Protected)
        ? 'Partially protected'
        : hasBindMounts
          ? 'Storage unprotected'
          : 'Volumes unprotected',
    };
  }
  if (aggregate === Status.Protected) {
    return {
      Icon: ShieldCheck,
      className: 'text-emerald-600 dark:text-emerald-400',
      label: hasBindMounts ? 'All storage targets protected' : 'All volumes protected',
    };
  }
  return {
    ...neutral,
    label: statuses.includes(null) || statuses.includes(undefined) ? 'Coverage unavailable' : 'Some storage excluded',
  };
}

export function ResourceBackupIndicator({
  source,
  coverage,
  compact = false,
  returnTo,
}: {
  source: ResourceBackupSource;
  coverage?: ResourceBackupSummaryView | null;
  compact?: boolean;
  returnTo?: string;
}) {
  const [open, setOpen] = useState(false);
  const pinned = useRef(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const closeTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const skipFocusOpen = useRef(false);
  const cardId = useId();
  const titleId = useId();
  const { Icon, className, label } = coverageAppearance(coverage);
  const target =
    source.$type === 'DockerVolume' ? source.volumeName : source.$type === 'BindMount' ? source.hostPath : undefined;

  const cancelClose = () => clearTimeout(closeTimer.current);
  useEffect(() => () => clearTimeout(closeTimer.current), []);

  const closeAfterLeave = () => {
    cancelClose();
    closeTimer.current = setTimeout(() => {
      if (
        !pinned.current &&
        !trigger.current?.matches(':focus') &&
        !content.current?.contains(document.activeElement)
      ) {
        setOpen(false);
      }
    }, 200);
  };

  const changeOpen = (value: boolean) => {
    cancelClose();
    pinned.current = false;
    setOpen(value);
  };

  return (
    <Popover open={open} onOpenChange={changeOpen}>
      <PopoverAnchor asChild>
        <Button
          ref={trigger}
          type="button"
          variant="ghost"
          size="icon"
          className={cn('shrink-0', compact ? 'size-6' : 'size-8', className)}
          aria-label={target ? `Backups for ${target}: ${label}` : `Backups: ${label}`}
          aria-haspopup="dialog"
          aria-expanded={open}
          aria-controls={open ? cardId : undefined}
          onPointerEnter={(event) => {
            if (event.pointerType === 'touch') return;
            cancelClose();
            setOpen(true);
          }}
          onPointerLeave={closeAfterLeave}
          onFocus={() => {
            if (!skipFocusOpen.current) setOpen(true);
            skipFocusOpen.current = false;
          }}
          onBlur={closeAfterLeave}
          onClick={() => {
            cancelClose();
            const next = !pinned.current;
            pinned.current = next;
            setOpen(next);
          }}
          onKeyDown={(event) => {
            if (event.key === 'ArrowDown' || (event.key === 'Tab' && !event.shiftKey && open)) {
              event.preventDefault();
              content.current?.focus();
            }
          }}>
          <Icon className={cn(compact ? 'size-3.5' : 'size-4', className)} aria-hidden="true" />
        </Button>
      </PopoverAnchor>
      <PopoverContent
        ref={content}
        id={cardId}
        aria-labelledby={titleId}
        tabIndex={-1}
        align="end"
        sideOffset={8}
        collisionPadding={12}
        className="max-h-[min(32rem,var(--radix-popover-content-available-height))] w-[min(26rem,calc(100vw-1.5rem))] overflow-y-auto overscroll-contain rounded-xl p-0 shadow-lg"
        onOpenAutoFocus={(event) => event.preventDefault()}
        onCloseAutoFocus={(event) => event.preventDefault()}
        onPointerEnter={cancelClose}
        onPointerLeave={closeAfterLeave}
        onFocus={cancelClose}
        onBlur={closeAfterLeave}
        onEscapeKeyDown={() => {
          if (document.activeElement !== trigger.current) {
            skipFocusOpen.current = true;
            trigger.current?.focus();
          }
        }}
        onInteractOutside={(event) => {
          if (event.target instanceof Node && trigger.current?.contains(event.target)) event.preventDefault();
        }}>
        <div className="flex flex-wrap items-center gap-3 border-b bg-muted/20 px-4 py-3">
          <span className="flex size-8 shrink-0 items-center justify-center rounded-lg border bg-background">
            <Icon className={cn('size-4', className)} aria-hidden="true" />
          </span>
          <h2 id={titleId} className="text-sm font-semibold">
            Backups
          </h2>
          {!!coverage?.volumes.length && !coverage.unavailable && (
            <Badge variant="secondary" className={cn('ml-auto whitespace-normal font-medium', className)}>
              {label}
            </Badge>
          )}
        </div>
        <div className="p-4">
          <ResourceBackupSummary source={source} coverage={coverage} returnTo={returnTo} />
        </div>
      </PopoverContent>
    </Popover>
  );
}
