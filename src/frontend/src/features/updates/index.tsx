import { useId, useState } from 'react';
import { ChevronDown, Info, RefreshCw } from 'lucide-react';
import { ActivityResourceType } from '@/api/generated/api.types';
import { UpdateCheckSettings } from './config';
import { UpdateProgress } from './installation';
import { AgentStatusBadge, AgentSummaryBadge, CoreStatusBadge, DatabaseStatusBadge } from './status-badge';
import { Timestamp } from '@/components/custom/timestamp';
import { Button } from '@/components/ui/button';
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
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
  showTaskSheet: true,
  tabsVariant: 'bordered',
  header: {
    subtitle: 'Update Citadel and its Agents, configure scheduled updates, and review activity.',
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
  const agents = useRead('getAgentUpdates');
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
          aria-label="Update information"
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
        aria-label="Update information"
        align="end"
        sideOffset={8}
        collisionPadding={12}
        className="w-[min(20rem,calc(100vw-1.5rem))] overflow-hidden rounded-lg p-0 shadow-md">
        <div className="border-b bg-muted/20 px-4 py-3">
          <h2 className="text-sm font-semibold">Update information</h2>
        </div>
        {status ? (
          <dl className="space-y-3 p-4 text-sm">
            <div className="flex items-center justify-between gap-3">
              <dt className="font-medium">Citadel Core</dt>
              <dd>
                <CoreStatusBadge status={status} failed={isError} />
              </dd>
            </div>
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
        <div className="border-t p-4 text-xs">
          <div className="flex items-center justify-between gap-3">
            <h3 className="font-medium">Database</h3>
            <DatabaseStatusBadge status={status} />
          </div>
          <p className="mt-2 font-mono text-muted-foreground">PostgreSQL {status?.databaseVersion ?? '—'}</p>
        </div>
        <Collapsible className="border-t p-4 text-xs">
          <CollapsibleTrigger className="group flex w-full items-center gap-2 text-left">
            <span className="mr-auto font-medium">Agents</span>
            <AgentSummaryBadge state={agents.data?.data} failed={agents.isError} />
            <ChevronDown
              aria-hidden="true"
              className="size-3.5 shrink-0 text-muted-foreground transition-transform group-data-[state=open]:rotate-180"
            />
          </CollapsibleTrigger>
          <CollapsibleContent>
            {agents.data?.data.checkError && (
              <p className="mt-2 text-muted-foreground">{agents.data.data.checkError}</p>
            )}
            {!!agents.data?.data.targets.length && (
              <ul className="mt-3 max-h-56 space-y-3 overflow-y-auto">
                {agents.data.data.targets.map((target) => (
                  <li key={target.id} className="space-y-1">
                    <div className="flex justify-between gap-3">
                      <span className="min-w-0 break-words font-medium">{target.name}</span>
                      <AgentStatusBadge status={target.status} />
                    </div>
                    <p className="break-all font-mono text-muted-foreground">
                      {target.installedVersion ?? 'Version unknown'}
                      {target.status === 'Available' && target.availableVersion && ` → ${target.availableVersion}`}
                    </p>
                    {target.nodes > 1 && <p className="text-muted-foreground">{target.nodes} nodes</p>}
                    {target.reason && <p className="text-muted-foreground">{target.reason}</p>}
                  </li>
                ))}
              </ul>
            )}
          </CollapsibleContent>
        </Collapsible>
      </HoverCardContent>
    </HoverCard>
  );
}

function UpdateStatus() {
  const { data } = useRead('getInstanceUpdates');
  return data ? <UpdateProgress status={data.data} /> : null;
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
