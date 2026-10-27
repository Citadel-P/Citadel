import { Timestamp } from '@/components/custom/timestamp';
import { useId, useState } from 'react';
import { ChevronDown } from 'lucide-react';
import { AutoUpdateStatus, ImageUpdateStatus as Status, type ImageTagUpdate } from '@/api/generated/api.types';
import { UpdateStatusIcon } from '@/components/custom/common';
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import { Button } from '@/components/ui/button';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';

const labels: Record<Status, string> = {
  UpToDate: 'Up to date',
  UpdateAvailable: 'Update available',
  NotChecked: 'Not checked',
  CheckFailed: 'Check failed',
  NotApplicable: 'Not applicable',
  Unknown: 'Cannot determine',
};
const icons: Partial<Record<Status, AutoUpdateStatus>> = {
  UpToDate: AutoUpdateStatus.UpToDate,
  UpdateAvailable: AutoUpdateStatus.UpdateAvailable,
  CheckFailed: AutoUpdateStatus.Failed,
};
function statusIcon(update: ImageTagUpdate) {
  return update.isStale ? AutoUpdateStatus.Unknown : (icons[update.status] ?? AutoUpdateStatus.Unknown);
}
function statusExplanation(update: ImageTagUpdate) {
  switch (update.status) {
    case Status.UpdateAvailable:
      return 'The registry has a different image for this tag. Use Pull latest to download it to this host. Existing workloads keep using their current image.';
    case Status.UpToDate:
      return 'This image matches the registry for the selected tag and platform.';
    default:
      return update.reason ?? 'Check for updates to compare this image with the registry.';
  }
}
export function ImageUpdateStatus({
  updates,
  detail = false,
  compact = false,
}: {
  updates?: ImageTagUpdate[] | null;
  detail?: boolean;
  compact?: boolean;
}) {
  const format = useProfileDateTimeFormatter();
  const [open, setOpen] = useState(false);
  const contentId = useId();
  const entries = (
    <div className="space-y-3">
      {!updates?.length && <p className="text-sm text-muted-foreground">No registry update check has been recorded.</p>}
      {updates?.map((update) => (
        <div key={update.reference} className="space-y-2 text-sm">
          <div className="flex flex-wrap items-center gap-2">
            <UpdateStatusIcon updateStatus={statusIcon(update)} />
            <span className="font-medium">{labels[update.status] ?? 'Cannot determine'}</span>
            {update.isStale && <span className="text-muted-foreground">Stale observation</span>}
          </div>
          {update.reference && <p className="break-all font-mono text-xs">{update.reference}</p>}
          <p className="text-muted-foreground">{statusExplanation(update)}</p>
          {update.isStale && (
            <p className="text-muted-foreground">This result may be out of date. Check again to confirm.</p>
          )}
          {update.lastAttemptAt && (
            <p className="text-xs text-muted-foreground">
              Last check: <Timestamp value={update.lastAttemptAt} formatDateTime={format} className="text-inherit" />
            </p>
          )}
          {update.lastSuccessAt && update.status !== 'UpToDate' && update.status !== 'UpdateAvailable' && (
            <p>
              Last successful observation:{' '}
              <Timestamp value={update.lastSuccessAt} formatDateTime={format} className="text-inherit" />
            </p>
          )}
          {(update.localConfigDigest || update.remoteConfigDigest || update.remoteManifestDigest) && (
            <Collapsible>
              <CollapsibleTrigger asChild>
                <Button variant="ghost" size="sm" className="group h-auto gap-2 px-0 py-1 font-normal">
                  Technical details
                  <ChevronDown className="size-3 transition-transform group-data-[state=open]:rotate-180" />
                </Button>
              </CollapsibleTrigger>
              <CollapsibleContent className="space-y-2 pt-2">
                <p className="text-xs text-muted-foreground">
                  Configuration digests identify image content. Different digests do not necessarily mean a newer
                  software version.
                </p>
                <dl className="space-y-2 text-xs">
                  {(
                    [
                      ['Local configuration', update.localConfigDigest],
                      ['Registry configuration', update.remoteConfigDigest],
                      ['Registry manifest', update.remoteManifestDigest],
                    ] as const
                  ).map(
                    ([label, digest]) =>
                      digest && (
                        <div key={label}>
                          <dt className="text-muted-foreground">{label}</dt>
                          <dd className="break-all font-mono">{digest}</dd>
                        </div>
                      ),
                  )}
                </dl>
              </CollapsibleContent>
            </Collapsible>
          )}
        </div>
      ))}
    </div>
  );
  if (detail) return entries;
  const states = new Set(updates?.map((update) => update.status));
  const stale = updates?.some((update) => update.isStale);
  const summary = !updates?.length
    ? 'Not checked'
    : states.size > 1
      ? 'Mixed status'
      : (labels[updates[0].status] ?? 'Cannot determine');
  const label = stale ? `${summary} (stale)` : summary;
  // A green summary requires every tag to have a fresh, successful comparison.
  const icon = states.has(Status.CheckFailed)
    ? AutoUpdateStatus.Failed
    : states.has(Status.UpdateAvailable) && !stale
      ? AutoUpdateStatus.UpdateAvailable
      : states.size === 1 && states.has(Status.UpToDate) && !stale
        ? AutoUpdateStatus.UpToDate
        : AutoUpdateStatus.Unknown;
  return (
    <HoverCard open={open} onOpenChange={setOpen} openDelay={150} closeDelay={150}>
      <HoverCardTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size={compact ? 'icon' : 'sm'}
          className={compact ? 'size-6' : 'h-auto gap-2 px-0 py-1 font-normal'}
          aria-label={`Update status: ${label}`}
          aria-expanded={open}
          aria-controls={open ? contentId : undefined}
          onClick={() => setOpen(true)}
          onTouchStart={() => setOpen(true)}
          onKeyDown={(event) => {
            if (event.key === 'Escape') setOpen(false);
          }}>
          <UpdateStatusIcon updateStatus={icon} />
          {!compact && <span>{label}</span>}
        </Button>
      </HoverCardTrigger>
      <HoverCardContent
        id={contentId}
        role="region"
        aria-label="Image update details"
        align="start"
        className="max-h-96 w-96 max-w-[calc(100vw-2rem)] overflow-auto border-border bg-background p-4 shadow-lg">
        <h4 className="mb-3 text-sm font-medium">Update status</h4>
        {entries}
      </HoverCardContent>
    </HoverCard>
  );
}
