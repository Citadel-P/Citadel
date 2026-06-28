import { DataTable } from '@/components/ui/data-table';
import {
  AutoUpdateStatus,
  ImageUpdateState,
  RecreateStackOnNewCommitState,
  ResourceControlState,
  StackSource,
  StackView,
} from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useMemo } from 'react';
import { useNavigate } from 'react-router';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { ContentCard } from '@/components/custom/content-card';
import { PlatformStatusCell, UPDATE_STATUS_UI, UpdateStatusIcon } from '@/components/custom/common';
import { truncate } from '@/lib/truncate';
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import { fromNow } from '@/lib/dayjs.helper';
import { formatId } from '@/lib/utils';
import { FileText, GitBranch } from 'lucide-react';

export const StacksTable = ({
  items,
  actions,
  isLoading,
}: {
  items: StackView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: StackView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [_, setSelectedResources] = useSelectedResources<StackView>('Stack');
  const cols = useMemo(() => columns(actions ?? {}), [actions]);

  return (
    <ContentCard>
      <DataTable columns={cols} data={items ?? []} isLoading={isLoading} onSelectionChange={setSelectedResources} />
    </ContentCard>
  );
};

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: StackView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
): ColumnDef<StackView>[] => [
  {
    id: 'select',
    header: ({ table }) => (
      <Checkbox
        checked={table.getIsAllPageRowsSelected() || (table.getIsSomePageRowsSelected() && 'indeterminate')}
        onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
        aria-label="Select all"
      />
    ),
    cell: ({ row }) => (
      <Checkbox
        checked={row.getIsSelected()}
        onCheckedChange={(value) => row.toggleSelected(!!value)}
        aria-label="Select stack"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <StackNameRow stack={row.original} />,
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.name?.localeCompare(rowB.original?.name),
  },
  {
    accessorKey: 'source',
    header: ({ column }) => <SortableCell cellName="Source" column={column} />,
    cell: ({ row }) => <StackSourceCell stack={row.original} />,
    sortingFn: (rowA, rowB) => getStackSourceLabel(rowA.original).localeCompare(getStackSourceLabel(rowB.original)),
  },
  {
    accessorKey: 'updateStatus',
    header: ({ column }) => <SortableCell cellName="Update Status" column={column} />,
    cell: ({ row }) => <StackUpdateStatusCell stack={row.original} />,
    sortingFn: (rowA, rowB) =>
      getStackUpdateStatus(rowA.original).localeCompare(getStackUpdateStatus(rowB.original)),
  },

  {
    accessorKey: 'platform',
    header: ({ column }) => <SortableCell cellName="Platform" column={column} />,
    cell: ({ row }) => (
      <PlatformStatusCell
        status={row.original.platformStatus!}
        id={row.original.platformId!}
        name={row.original.platformName ?? ''}
      />
    ),
    sortingFn: (rowA, rowB) => (rowA.original.platformName! < rowB.original.platformName! ? 1 : -1),
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

const StackNameRow = ({ stack }: { stack: StackView }) => {
  const navigate = useNavigate();
  function onClick() {
    navigate(`/stacks/edit/${stack.id}/`);
  }
  return (
    <div className="flex items-center whitespace-nowrap">
      <div className="flex items-center">
        <StateIndicator value={stack.status} isProcessing={stack.controlState === ResourceControlState.Processing} />
      </div>
      <span
        className="cursor-pointer hover:underline"
        onClick={onClick}
        title={stack.name}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            onClick();
          }
        }}
        tabIndex={0}
        role="button"
        aria-label="Show stack details">
        {truncate(stack.name ?? '', 32, 'right')}
      </span>
    </div>
  );
};

const StackSourceCell = ({ stack }: { stack: StackView }) => {
  if (stack.stackSource === StackSource.Git) {
    const repositoryName = stack.source?.gitRepositoryName ?? 'Git repository';
    const branch = stack.source?.branch;
    const label = branch ? `${repositoryName}/${branch}` : repositoryName;

    return (
      <span className="flex min-w-0 items-center gap-2 text-sm " title={label}>
        <GitBranch className="size-3.5 shrink-0 text-foreground/80" />
        <span className="truncate">{label}</span>
      </span>
    );
  }

  return (
    <span className="flex min-w-0 items-center gap-2 text-sm" title="UI defined">
      <FileText className="size-3.5 shrink-0 text-foreground/80" />
      <span className="truncate">UI defined</span>
    </span>
  );
};

const StackUpdateStatusCell = ({ stack }: { stack: StackView }) => {
  const states = getStackImageUpdateStates(stack);
  const gitState = getStackGitUpdateState(stack);
  const status = getStackUpdateStatus(stack);
  const { label } = UPDATE_STATUS_UI[status];

  if (status === AutoUpdateStatus.Unknown) {
    return <span className="text-muted-foreground text-sm">{'<none>'}</span>;
  }

  const updatedAt = getLatestStackUpdateCheckTime(stack);

  return (
    <HoverCard openDelay={150} closeDelay={150}>
      <HoverCardTrigger asChild>
        <div className="inline-flex cursor-default items-center gap-2">
          <UpdateStatusIcon updateStatus={status} />
          <span>{label}</span>
        </div>
      </HoverCardTrigger>
      <HoverCardContent align="start" className="w-96 p-4 shadow-lg border-border bg-background">
        <div className="flex justify-between items-start mb-4">
          <div className="space-y-1">
            <h4 className="text-sm font-medium leading-none text-foreground">{stack.name}</h4>
            {updatedAt && <p className="text-xs text-muted-foreground">Checked {fromNow(new Date(updatedAt))}</p>}
          </div>
          <UpdateStatusIcon updateStatus={status} />
        </div>

        <div className="space-y-3">
          {gitState && <StackGitUpdateStateRow stack={stack} state={gitState} />}
          {states.slice(0, 4).map((state) => (
            <StackUpdateStateRow key={`${state.serviceName}:${state.imageName}`} state={state} />
          ))}
          {states.length > 4 && <p className="text-xs text-muted-foreground">+{states.length - 4} more services</p>}
        </div>
      </HoverCardContent>
    </HoverCard>
  );
};

const StackGitUpdateStateRow = ({
  stack,
  state,
}: {
  stack: StackView;
  state: RecreateStackOnNewCommitState;
}) => {
  const source = stack.source;
  const remoteCommit = state.remoteCommitSha ?? undefined;

  return (
    <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-sm">
      <span className="text-muted-foreground text-xs">Repository</span>
      <span className="truncate" title={source?.gitRepositoryName ?? undefined}>
        {source?.gitRepositoryName ?? '-'}
      </span>
      <span className="text-muted-foreground text-xs">Branch</span>
      <span className="truncate" title={source?.branch ?? undefined}>
        {source?.branch ?? '-'}
      </span>
      <span className="text-muted-foreground text-xs">Current</span>
      <span className="font-mono text-xs text-foreground/80 truncate" title={state.currentCommitSha}>
        {formatId(state.currentCommitSha)}
      </span>
      {remoteCommit && (
        <>
          <span className="text-muted-foreground text-xs">Available</span>
          <span className="font-mono text-xs text-amber-600 dark:text-amber-500 truncate" title={remoteCommit}>
            {formatId(remoteCommit)}
          </span>
        </>
      )}
    </div>
  );
};

const StackUpdateStateRow = ({ state }: { state: ImageUpdateState }) => {
  return (
    <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-sm">
      <span className="text-muted-foreground text-xs">Service</span>
      <span className="truncate" title={state.serviceName}>
        {state.serviceName}
      </span>
      <span className="text-muted-foreground text-xs">Image</span>
      <span className="truncate text-xs" title={state.imageName}>
        {truncate(state.imageName, 36)}
      </span>
      <span className="text-muted-foreground text-xs">Current</span>
      <span className="font-mono text-xs text-foreground/80 truncate" title={state.currentDigest}>
        {formatId(state.currentDigest)}
      </span>
      {state.updateAvailable && (
        <>
          <span className="text-muted-foreground text-xs">Available</span>
          <span className="font-mono text-xs text-amber-600 dark:text-amber-500 truncate" title={state.remoteDigest ?? undefined}>
            {formatId(state.remoteDigest ?? undefined)}
          </span>
        </>
      )}
    </div>
  );
};

const getStackImageUpdateStates = (stack: StackView): ImageUpdateState[] =>
  stack.stackUpdateState?.recreateStackOnNewImageState?.autoUpdateStates ?? [];

const getStackSourceLabel = (stack: StackView): string => {
  if (stack.stackSource !== StackSource.Git) return 'UI defined';

  const repositoryName = stack.source?.gitRepositoryName ?? 'Git repository';
  return stack.source?.branch ? `${repositoryName}/${stack.source.branch}` : repositoryName;
};

const getLatestStackUpdateCheckTime = (stack: StackView): number | undefined => {
  const candidates = [
    getStackGitUpdateState(stack)?.lastCheckedAt,
    ...getStackImageUpdateStates(stack).map((state) => state.lastCheckedAt),
  ];

  return candidates
    .map((value) => (value ? new Date(value).getTime() : Number.NaN))
    .filter((value) => !Number.isNaN(value))
    .sort((a, b) => b - a)[0];
};

const getStackGitUpdateState = (stack: StackView): RecreateStackOnNewCommitState | null => {
  const state = stack.stackUpdateState;
  if (!state || !('recreateStackOnNewCommitState' in state)) return null;

  const gitState = state.recreateStackOnNewCommitState;
  return gitState?.currentCommitSha ? gitState : null;
};

const getStackUpdateStatus = (stack: StackView): AutoUpdateStatus => {
  const gitState = getStackGitUpdateState(stack);
  if (gitState?.remoteCommitSha && gitState.remoteCommitSha !== gitState.currentCommitSha) {
    return AutoUpdateStatus.UpdateAvailable;
  }

  if (gitState?.currentCommitSha) return AutoUpdateStatus.UpToDate;

  const states = getStackImageUpdateStates(stack);
  if (states.length === 0) return AutoUpdateStatus.Unknown;
  if (states.some((state) => state.updateAvailable)) return AutoUpdateStatus.UpdateAvailable;
  return AutoUpdateStatus.UpToDate;
};
