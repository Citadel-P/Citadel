import { useId, useState } from 'react';
import { ExternalLink, Info, RefreshCw } from 'lucide-react';
import { ActivityResourceType } from '@/api/generated/api.types';
import { UpdateCheckSettings } from './config';
import { InstallUpdate, UpdateProgress } from './installation';
import { Timestamp } from '@/components/custom/timestamp';
import { AlertMessage } from '@/components/custom/alert-message';
import { Button } from '@/components/ui/button';
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import Loader from '@/components/ui/loader';
import { useRead } from '@/lib/hooks';
import type { TabbedResourceComponents } from '@/pages/types';
import type { ActivityQueryState } from '@/lib/atoms';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { useScopedActivitiesGroup } from '@/features/activities/hooks/useActivitiesGroup';
import { ScopedActivitiesTable } from '@/features/activities/table';

export const UpdatesComponents: TabbedResourceComponents = {
  Icon: RefreshCw,
  access: 'administrator',
  showTaskSheet: false,
  tabsVariant: 'bordered',
  header: {
    subtitle: 'Manage Citadel release checks and review update activity.',
    variant: 'plain',
    showSearch: false,
    showAdd: false,
  },
  SubHeader: UpdateStatus,
  TabActions: UpdateInfo,
  Tabs: [
    { label: 'Config', slug: 'config', Content: UpdateCheckSettings },
    { label: 'Activities', slug: 'activities', Content: InstanceActivities },
  ],
};

function UpdateInfo() {
  const { data, isError } = useRead('getInstanceUpdates');
  const formatDate = useProfileDateTimeFormatter();
  const [open, setOpen] = useState(false);
  const cardId = useId();
  const status = data?.data;
  return (
    <HoverCard open={open} onOpenChange={setOpen} openDelay={150} closeDelay={200}>
      <HoverCardTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-8 text-muted-foreground"
          aria-label="Instance information"
          aria-expanded={open}
          aria-controls={open ? cardId : undefined}
          onClick={() => setOpen(true)}
          onKeyDown={(event) => {
            if (event.key === 'Escape') setOpen(false);
          }}>
          <Info className="size-4" aria-hidden="true" />
        </Button>
      </HoverCardTrigger>
      <HoverCardContent
        id={cardId}
        role="region"
        aria-label="Instance information"
        align="end"
        sideOffset={8}
        collisionPadding={12}
        className="w-[min(20rem,calc(100vw-1.5rem))] overflow-hidden rounded-lg p-0 shadow-md">
        <div className="border-b bg-muted/20 px-4 py-3">
          <h2 className="text-sm font-semibold">Instance information</h2>
        </div>
        {status ? (
          <dl className="space-y-3 p-4 text-sm">
            <div className="space-y-1.5">
              <dt className="text-xs text-muted-foreground">Installed version</dt>
              <dd className="rounded-md bg-muted/40 px-2.5 py-2 font-mono text-xs leading-relaxed break-all">
                Citadel {status.installedVersion}
              </dd>
            </div>
            <div className="flex items-center justify-between gap-3">
              <dt className="text-xs text-muted-foreground">Release channel</dt>
              <dd className="rounded-md border bg-muted/20 px-2 py-0.5 text-xs font-medium">{status.channel}</dd>
            </div>
            <div className="space-y-1 border-t pt-3">
              <dt className="text-xs text-muted-foreground">Last successful check</dt>
              <dd className="text-xs leading-relaxed tabular-nums">
                {status.lastSuccess ? (
                  <Timestamp value={status.lastSuccess} formatDateTime={formatDate} className="text-xs" />
                ) : (
                  'Not checked'
                )}
              </dd>
            </div>
          </dl>
        ) : (
          <p className="p-4 text-sm text-muted-foreground">
            {isError ? 'Information unavailable.' : 'Loading instance information…'}
          </p>
        )}
      </HoverCardContent>
    </HoverCard>
  );
}

function UpdateStatus() {
  const status = useRead('getInstanceUpdates');
  const data = status.data?.data;
  if (status.isPending) return <Loader />;
  if (!data)
    return (
      <div>
        <p role="alert">Could not load update status.</p>
        <Button onClick={() => void status.refetch()}>Retry</Button>
      </div>
    );
  return (
    <div className="space-y-2" aria-live="polite">
      <UpdateProgress status={data} />
      {!data.operation && (
        <>
          {data.available ? (
            <AlertMessage
              type="info"
              title={`Citadel ${data.available.version} is available.`}
              date={data.lastSuccess ?? undefined}
              className="my-0 flex-wrap"
              action={
                <div className="flex items-center gap-2">
                  <InstallUpdate status={data} />
                  {data.installationUnavailableReason && (
                    <Button variant="outline" size="sm" asChild>
                      <a
                        href="https://docs.citadelplane.com/operations/upgrade-and-rollback/"
                        target="_blank"
                        rel="noreferrer">
                        Upgrade guide <ExternalLink className="size-3.5" aria-hidden="true" />
                      </a>
                    </Button>
                  )}
                  <Button variant="outline" size="sm" className="text-foreground" asChild>
                    <a href={data.available.releaseUrl} target="_blank" rel="noreferrer">
                      Release notes <ExternalLink className="size-3.5" aria-hidden="true" />
                    </a>
                  </Button>
                </div>
              }>
              {data.installationUnavailableReason ?? 'Review the release notes before upgrading.'}
            </AlertMessage>
          ) : (
            !data.checkError && (
              <AlertMessage type="info" className="my-0 flex-wrap" date={data.lastSuccess ?? undefined}>
                {data.lastSuccess ? 'No newer release found at the last check.' : 'No successful release check yet.'}
              </AlertMessage>
            )
          )}
          {data.checkError && (
            <div role="alert">
              <AlertMessage
                type="warning"
                title="Update check failed"
                className="my-0 flex-wrap"
                date={data.lastAttempt ?? undefined}>
                {data.checkError} Last successful release information is retained.
              </AlertMessage>
            </div>
          )}
        </>
      )}
    </div>
  );
}

function InstanceActivities() {
  const status = useRead('getInstanceUpdates');
  if (status.isPending) return <Loader />;
  if (!status.data) return null; // The shared status panel owns read errors and retry.
  return <UpdateActivities key={status.data.data.instanceId} instanceId={status.data.data.instanceId} />;
}

function UpdateActivities({ instanceId }: { instanceId: string }) {
  const [query, setQuery] = useState<ActivityQueryState>({
    resourceType: ActivityResourceType.Updates,
    resourceId: instanceId,
    eventType: 'All',
    page: 1,
    pageSize: 25,
  });
  const result = useScopedActivitiesGroup(query);
  if (result.error)
    return (
      <div role="alert">
        Could not load update activities.{' '}
        <Button variant="outline" onClick={() => void result.refetch()}>
          Retry
        </Button>
      </div>
    );
  return (
    <ScopedActivitiesTable
      pagedResult={result.pagedActivities!}
      isLoading={result.isLoading}
      displayPagging
      query={query}
      setQuery={(patch) =>
        setQuery((previous) => ({
          ...previous,
          ...patch,
          resourceType: ActivityResourceType.Updates,
          resourceId: instanceId,
        }))
      }
    />
  );
}
