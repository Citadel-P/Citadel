import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { useState, type ReactNode } from 'react';
import { RefreshCw } from 'lucide-react';
import type { StockFeatures, ColumnDef } from '@tanstack/react-table';
import { DataTable } from '@/components/ui/data-table';
import { StateIndicator } from '@/components/custom/state-indicator';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import { Link } from 'react-router';
import { useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';
import { useAppContext } from '@/lib/context/app-context';
import { Button } from '@/components/ui/button';
import type { ImageUsage as ImageUsageView, ImageUsageEntry } from '@/api/generated/api.types';

function workloadRoute(entry: ImageUsageEntry) {
  const route =
    entry.workloadKind === 'Deployment' ? 'deployments' : entry.workloadKind === 'Stack' ? 'stacks' : 'swarm-services';
  return `/${route}/edit/${entry.workloadId}`;
}
function referenceDescription(entry: ImageUsageEntry) {
  return entry.relationship === 'Container'
    ? `Container: ${entry.containerName ?? entry.containerId} (${entry.containerState ?? 'unknown'})`
    : entry.relationship === 'SavedLocalImage'
      ? 'Saved Local image dependency'
      : `Saved ${entry.relationship === 'SavedDigest' ? 'digest' : 'tag'} reference: ${entry.reference}`;
}
function ImageUsageTable({ usage }: { usage: ImageUsageView }) {
  const formatDateTime = useProfileDateTimeFormatter();
  type Row = ImageUsageEntry & { id: string };
  const columns: ColumnDef<StockFeatures, Row>[] = [
    {
      accessorKey: 'workloadName',
      header: 'Name',
      cell: ({ row }) => (
        <div
          className="flex flex-wrap items-center gap-1"
          aria-label={`${row.original.workloadName ?? 'Workload'}: ${row.original.workloadStatus ?? 'Unknown'}`}>
          <StateIndicator value={row.original.workloadStatus ?? 'Unknown'} />
          <Link className="table-link" to={workloadRoute(row.original)}>
            {row.original.workloadName ?? row.original.workloadId}
          </Link>
        </div>
      ),
    },
    {
      id: 'reference',
      header: 'Reference',
      cell: ({ row }) => <span className="break-all">{referenceDescription(row.original)}</span>,
    },
    {
      accessorKey: 'observedAt',
      header: 'Observed',
      cell: ({ row }) => (
        <div className="space-y-1">
          <TimestampCell value={row.original.observedAt} formatDateTime={formatDateTime} />
          {row.original.isStale && <p className="text-warning">Inventory observation is stale.</p>}
        </div>
      ),
    },
  ];
  const groups = [
    { kind: 'Deployment', title: 'Deployments' },
    { kind: 'Stack', title: 'Stacks' },
    { kind: 'SwarmService', title: 'Swarm Services' },
  ];
  return (
    <div className="min-w-0 space-y-4">
      {groups.map(({ kind, title }) => {
        const entries = usage.entries
          .filter((entry) => entry.workloadKind === kind)
          .map((entry, index) => ({
            ...entry,
            id: `${entry.relationship}-${entry.workloadId}-${entry.containerId}-${index}`,
          }));
        if (entries.length === 0) return null;
        return (
          <section key={kind} aria-label={title} className="min-w-0 space-y-3">
            <h3 className="text-sm font-medium">{title}</h3>
            <DataTable columns={columns} data={entries} isLoading={false} />
          </section>
        );
      })}
    </div>
  );
}

export function ImageUsageEntries({ usage }: { usage: ImageUsageView }) {
  const formatDateTime = useProfileDateTimeFormatter();
  return (
    <div className="space-y-3 text-sm">
      {usage.entries.length === 0 && (
        <p>
          {usage.nextOffset != null
            ? 'No matches yet. Continue to check more references.'
            : 'No known references on this page.'}
        </p>
      )}
      {usage.entries.map((entry, index) => {
        return (
          <div
            key={`${entry.relationship}-${entry.workloadId}-${entry.containerId}-${index}`}
            className="rounded-sm border p-3 space-y-1">
            {entry.workloadId ? (
              <Link className="font-medium underline" to={workloadRoute(entry)}>
                {entry.workloadName}
              </Link>
            ) : (
              <span className="font-medium">{entry.containerName ?? 'Container'}</span>
            )}
            <p>{referenceDescription(entry)}</p>
            {entry.observedAt && <p className="text-muted-foreground">Observed {formatDateTime(entry.observedAt)}</p>}
            {entry.isStale && <p className="text-warning">Inventory observation is stale.</p>}
          </div>
        );
      })}
      {!usage.coverageComplete && (
        <p className="text-muted-foreground">
          Some image references may be missing. Confirm this image is no longer needed before deleting it.
        </p>
      )}
    </div>
  );
}

export function ImageUsage({
  imageId,
  dockerNodeId,
  initialUsage,
  managedOnly = false,
  children,
}: {
  imageId: string;
  dockerNodeId?: string | null;
  initialUsage?: ImageUsageView;
  managedOnly?: boolean;
  children?: (content: ReactNode, refresh: ReactNode) => ReactNode;
}) {
  const { currentPlatform } = useAppContext();
  const [offsets, setOffsets] = useState([0]);
  const offset = offsets[offsets.length - 1];
  const showingPreview = initialUsage !== undefined && offset === 0;
  const { data, isLoading, isFetching, isError, refetch } = useRead(
    'getImageUsage',
    {
      platformId: currentPlatform?.id ?? '',
      imageId,
      query: { dockerNodeId: dockerNodeId ?? undefined, offset },
    },
    { enabled: !showingPreview },
  );
  const usage = showingPreview ? initialUsage : data?.data;
  useDockerDaemonGroup(showingPreview ? undefined : currentPlatform?.id, {
    onImageEvent: ({ image }) => {
      if (image.dockerImageId === imageId && image.dockerNodeId === (dockerNodeId ?? null)) void refetch();
    },
    onContainerEvent: ({ container }) => {
      if (container.dockerImageId === imageId && container.dockerNodeId === (dockerNodeId ?? null)) void refetch();
    },
    onContainerStateChange: (patches) => {
      if (
        patches.some(
          (patch) =>
            (patch.dockerNodeId ?? null) === (dockerNodeId ?? null) &&
            usage?.entries.some((entry) => entry.containerId === patch.containerId),
        )
      )
        void refetch();
    },
    onSwarmNodeLocalResourcesUpdated: () => {
      if (dockerNodeId) void refetch();
    },
  });
  const refresh = !initialUsage && (
    <Button
      type="button"
      variant="ghost"
      size="icon"
      className="size-7"
      aria-label="Refresh references"
      title="Refresh references"
      disabled={isFetching}
      onClick={() => void refetch()}>
      <RefreshCw className={`size-4${isFetching ? ' animate-spin' : ''}`} aria-hidden="true" />
    </Button>
  );
  let content: ReactNode;
  if (!showingPreview && isLoading)
    content = <p className="text-sm text-muted-foreground">Loading image references…</p>;
  else if (!showingPreview && isError)
    content = (
      <div>
        <p>Image references could not be loaded.</p>
        <Button variant="outline" disabled={isFetching} onClick={() => void refetch()}>
          Retry
        </Button>
      </div>
    );
  else
    content = usage && (
      <div className="space-y-3">
        {managedOnly ? (
          <ImageUsageTable usage={{ ...usage, entries: usage.entries.filter((entry) => entry.workloadId) }} />
        ) : (
          <ImageUsageEntries usage={usage} />
        )}
        <div className="flex gap-2">
          {offset > 0 && (
            <Button
              variant="outline"
              disabled={isFetching}
              onClick={() => setOffsets((previous) => previous.slice(0, -1))}>
              Previous references
            </Button>
          )}
          {usage.nextOffset != null && (
            <Button
              variant="outline"
              disabled={isFetching}
              onClick={() => setOffsets((previous) => [...previous, usage.nextOffset!])}>
              More references
            </Button>
          )}
        </div>
      </div>
    );
  return children ? (
    children(content, refresh)
  ) : (
    <div className="space-y-3">
      {refresh}
      {content}
    </div>
  );
}
