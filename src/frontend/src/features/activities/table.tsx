import type { PublicActivityEventInfo, ActivityView, PagedActivityView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { StockFeatures, ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';
import { useActivityQuery, useTaskSheet } from '@/lib/atoms';
import { ActorCell, PagedDataTable } from '@/components/custom/common';
import { createResourceTargetColumn } from '@/components/custom/resource-target-column';
import { StateBadge } from '@/components/custom/state-badge';
import { Timestamp } from '@/components/custom/timestamp';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { formatActivityEvent } from '@/lib/utils';

const EMPTY_ROWS: ActivityView[] = [];

export const ActivitiesTable = ({
  pagedResult,
  isLoading,
  displayTarget = false,
  displayPagging = false,
}: {
  pagedResult: PagedActivityView;
  isLoading: boolean;
  displayTarget?: boolean;
  displayPagging?: boolean;
}) => {
  const [query, setQuery] = useActivityQuery();
  const formatDateTime = useProfileDateTimeFormatter();

  const cols = useMemo(() => columns(displayTarget, formatDateTime), [displayTarget, formatDateTime]);

  return (
    <PagedDataTable
      columns={cols}
      data={pagedResult?.items ?? EMPTY_ROWS}
      isLoading={isLoading}
      query={query}
      setQuery={setQuery}
      totalCount={pagedResult?.totalCount}
      showPagination={displayPagging}
    />
  );
};

const columns = (
  displayTarget: boolean,
  formatDateTime: DateTimeFormatter,
): ColumnDef<StockFeatures, ActivityView>[] => {
  const cols: ColumnDef<StockFeatures, ActivityView>[] = [
    {
      accessorKey: 'eventType',
      header: ({ column }) => <SortableCell cellName="Event Type" column={column} />,
      cell: ({ row }) => <EventTypeCell activity={row.original} />,
      sortFn: (rowA: any, rowB: any): number => rowA.original?.eventType?.localeCompare(rowB.original?.eventType),
    },
  ];

  if (displayTarget) {
    cols.push(
      createResourceTargetColumn({
        id: 'target',
        header: 'Target',
        getValue: (activity: ActivityView) => ({
          target: {
            resourceType: activity.resourceType,
            resourceId: activity.resourceId ?? undefined,
            resourceName: activity.resourceName,
          },
        }),
      }),
    );
  }

  cols.push(
    {
      accessorKey: 'status',
      header: ({ column }) => <SortableCell cellName="Status" column={column} />,
      cell: ({ row }) => <StateBadge value={row.original.status} kind="activity" />,
      sortFn: (rowA, rowB) => (rowA.original.status! < rowB.original.status! ? 1 : -1),
    },
    {
      accessorKey: 'createdAt',
      header: ({ column }) => <SortableCell cellName="Created" column={column} />,
      cell: ({ row }) => {
        return <Timestamp value={row.original.createdAt} formatDateTime={formatDateTime} />;
      },
      sortFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
    },
    {
      accessorKey: 'user',
      header: ({ column }) => <SortableCell cellName="User" column={column} />,
      cell: ({ row }) => <ActorCell type={row.original.actorType} name={row.original.actorName ?? ''} />,
      sortFn: (rowA, rowB) => (rowA.original.platformName! < rowB.original.platformName! ? 1 : -1),
    },
  );

  return cols;
};

function EventTypeCell({ activity }: { activity: ActivityView }) {
  const { open } = useTaskSheet('Activity');
  const summary = getActivitySummary(activity.info);

  return (
    <button
      type="button"
      onClick={() => open({ kind: 'activity', payload: activity })}
      className="table-link block min-w-0 max-w-96 cursor-pointer overflow-hidden text-left">
      <span className="flex min-w-0 flex-col gap-0.5">
        <span className="truncate">{formatActivityEvent(activity.eventType)}</span>
        {summary && (
          <span className="block truncate text-xs font-normal normal-case text-muted-foreground" title={summary}>
            {summary}
          </span>
        )}
      </span>
    </button>
  );
}

function getActivitySummary(info: PublicActivityEventInfo | null | undefined): string | null {
  if (!info?.$type) return null;

  switch (info.$type) {
    case 'BackupRunQueued':
    case 'BackupRunStarted':
      return info.trigger;
    case 'BackupRunCompleted':
      return [formatActivityEvent(info.status), info.errorMessage].filter(Boolean).join(' - ');
    case 'GitRepoWebhookReceived':
    case 'StackWebhookReceived':
    case 'BuildWebhookReceived':
    case 'ActionWebhookReceived':
    case 'BackupPolicyWebhookReceived':
      return [
        info.status,
        formatWebhookReason(info.reason),
        info.dispatchedBranch ? `queued ${info.dispatchedBranch}` : info.branch ? `branch ${info.branch}` : null,
        info.dispatchedCommitSha
          ? shortCommit(info.dispatchedCommitSha)
          : info.commitSha
            ? shortCommit(info.commitSha)
            : null,
      ]
        .filter(Boolean)
        .join(' - ');
    case 'SwarmServiceWebhookReceived':
      return [info.status, formatWebhookReason(info.reason)].filter(Boolean).join(' - ');
    case 'GitRepoPulled':
    case 'GitRepoCloned':
      return [info.result?.commitSha ? shortCommit(info.result.commitSha) : null, info.result?.message]
        .filter(Boolean)
        .join(' - ');
    case 'DeploymentDuplicated':
    case 'StackDuplicated':
    case 'SwarmServiceDuplicated':
      return `from ${info.source.resourceName}`;
    case 'BuildAgentPoolCreated':
    case 'BuildAgentPoolDeleted':
      return `${info.pool.provider} - ${info.pool.region} - ${info.pool.instanceType}`;
    case 'BuildAgentPoolUpdated':
      return `${info.oldPool.provider} - ${info.oldPool.region} - ${info.oldPool.instanceType}`;
    case 'BuildAgentPoolRenamed':
      return `${info.oldName} -> ${info.newName}`;
    case 'BuildAgentPoolConnected':
    case 'BuildAgentPoolDisconnected':
      return info.reason;
    case 'BuildAgentPoolTested':
      return [info.status, info.message].filter(Boolean).join(' - ');
    default:
      return null;
  }
}

function formatWebhookReason(reason: string | null | undefined) {
  switch (reason) {
    case 'No new commit':
      return 'already latest commit';
    case 'No relevant path changes':
      return 'no watched path changes';
    case 'Branch mismatch':
      return 'branch mismatch';
    case 'Unsupported event type':
      return 'unsupported event';
    default:
      return reason;
  }
}

function shortCommit(commit: string) {
  return commit.length > 12 ? commit.slice(0, 12) : commit;
}
