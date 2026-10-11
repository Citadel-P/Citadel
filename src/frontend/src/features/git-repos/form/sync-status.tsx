import { AuthorizedGitRepositoryView, GitRepositoryRefStatus, GitRepositorySyncMode } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { fromNow } from '@/lib/dayjs.helper';
import { ChevronDown, Clock } from 'lucide-react';
import { useId, useState } from 'react';

export function GitRepositorySyncStatus({ repository }: { repository: AuthorizedGitRepositoryView }) {
  const [open, setOpen] = useState(false);
  const titleId = useId();
  const reference = repository.syncStatus;
  const completed =
    reference?.status === GitRepositoryRefStatus.Healthy || reference?.status === GitRepositoryRefStatus.Degraded;
  const label =
    reference === undefined
      ? 'Check status unavailable'
      : reference?.status === GitRepositoryRefStatus.Healthy
        ? 'Check completed'
        : reference?.status === GitRepositoryRefStatus.Degraded
          ? 'Check failed'
          : reference?.status === GitRepositoryRefStatus.Syncing
            ? 'Checking now'
            : reference?.status === GitRepositoryRefStatus.Pending
              ? 'Check queued'
              : 'Not checked yet';
  const scheduled = repository.syncMode === GitRepositorySyncMode.PullInterval;
  const interval = repository.syncIntervalMinutes ?? 5;

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="xs"
          aria-label="Repository check details"
          className="text-muted-foreground">
          <Clock aria-hidden="true" />
          Checks
          <ChevronDown aria-hidden="true" />
        </Button>
      </PopoverTrigger>
      <PopoverContent
        align="end"
        sideOffset={8}
        collisionPadding={16}
        className="w-80 max-w-[calc(100vw-2rem)] p-0"
        aria-labelledby={titleId}>
        <div className="border-b px-4 py-3">
          <h3 id={titleId} className="text-sm font-medium">
            Repository checks
          </h3>
        </div>
        <dl className="space-y-3 p-4 text-sm">
          <div className="flex items-start justify-between gap-4">
            <dt className="text-muted-foreground">Branch</dt>
            <dd className="min-w-0 text-right">
              <code className="break-all text-xs">{repository.defaultBranch}</code>
            </dd>
          </div>
          <div className="flex justify-between gap-4">
            <dt className="text-muted-foreground">Status</dt>
            <dd className={reference?.status === GitRepositoryRefStatus.Degraded ? 'text-destructive' : undefined}>
              {label}
            </dd>
          </div>
          {completed && reference && (
            <div className="flex items-start justify-between gap-4">
              <dt className="shrink-0 text-muted-foreground">
                {reference.status === GitRepositoryRefStatus.Degraded ? 'Last attempt' : 'Last checked'}
              </dt>
              <dd className="text-right">
                <time dateTime={reference.lastSyncedAt}>
                  {fromNow(reference.lastSyncedAt)}
                  <span className="mt-0.5 block text-xs text-muted-foreground">
                    {new Date(reference.lastSyncedAt).toLocaleString()}
                  </span>
                </time>
              </dd>
            </div>
          )}
          <div className="flex justify-between gap-4">
            <dt className="text-muted-foreground">Schedule</dt>
            <dd
              className="text-right"
              title={scheduled ? 'The scheduler checks for due repositories once a minute.' : undefined}>
              {scheduled ? `Every ${interval} ${interval === 1 ? 'minute' : 'minutes'}` : 'Manual or webhook'}
            </dd>
          </div>
        </dl>
        {scheduled && (
          <p className="border-t px-4 py-3 text-xs leading-relaxed text-muted-foreground">
            Successful background syncs do not create activity entries.
          </p>
        )}
      </PopoverContent>
    </Popover>
  );
}
