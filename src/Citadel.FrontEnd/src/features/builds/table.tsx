import { BuildProjectView, BuildRunStatus, BuildRunView } from '@/api/generated/api.types';
import { ContentCard } from '@/components/custom/content-card';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { TagChips } from '@/features/tags/components';
import { useSelectedResources } from '@/lib/atoms';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useRead } from '@/lib/hooks';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { ActionData } from '@/pages/types';
import { ColumnDef } from '@tanstack/react-table';
import { GitBranch, Server } from 'lucide-react';
import { useMemo } from 'react';
import { Link } from 'react-router';

type ActionMap = Record<
  string,
  React.FC<{ resource: BuildProjectView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
>;

export function BuildsTable({
  items,
  actions,
  isLoading,
}: {
  items: BuildProjectView[];
  isLoading: boolean;
  actions: ActionMap;
}) {
  const [, setSelectedResources] = useSelectedResources<BuildProjectView>('Build');
  const formatDateTime = useProfileDateTimeFormatter();
  const { data: runsData } = useRead('listBuildRuns', { query: { limit: 100 } }, { refetchInterval: 5000 });
  const latestRuns = useMemo(() => indexLatestRuns(runsData?.data.runs ?? []), [runsData?.data.runs]);
  const cols = useMemo(() => columns(actions ?? {}, latestRuns, formatDateTime), [actions, formatDateTime, latestRuns]);

  return (
    <ContentCard>
      <DataTable columns={cols} data={items} isLoading={isLoading} onSelectionChange={setSelectedResources} />
    </ContentCard>
  );
}

const columns = (
  actions: ActionMap,
  latestRuns: Map<string, BuildRunView>,
  formatDateTime: DateTimeFormatter,
): ColumnDef<BuildProjectView>[] => [
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
        aria-label="Select build"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <BuildNameRow project={row.original} run={latestRuns.get(row.original.id)} />,
    sortingFn: (rowA, rowB) => rowA.original.name.localeCompare(rowB.original.name),
  },
  {
    accessorKey: 'branch',
    header: ({ column }) => <SortableCell cellName="Source" column={column} />,
    cell: ({ row }) => (
      <span className="inline-flex min-w-0 max-w-72 items-center gap-2 text-sm">
        <GitBranch className="size-3.5 shrink-0 text-muted-foreground" />
        <span className="truncate" title={row.original.branch}>{row.original.branch}</span>
      </span>
    ),
    sortingFn: (rowA, rowB) => rowA.original.branch.localeCompare(rowB.original.branch),
  },
  {
    accessorKey: 'platformId',
    header: ({ column }) => <SortableCell cellName="Platform" column={column} />,
    cell: ({ row }) => {
      const run = latestRuns.get(row.original.id);
      return (
        <span className="inline-flex min-w-0 max-w-72 items-center gap-2 text-sm">
          <Server className="size-3.5 shrink-0 text-muted-foreground" />
          <span className="truncate" title={run?.platformSnapshot.name ?? row.original.platformId}>
            {run?.platformSnapshot.name ?? row.original.platformId}
          </span>
        </span>
      );
    },
  },
  {
    accessorKey: 'imageRepository',
    header: ({ column }) => <SortableCell cellName="Image" column={column} />,
    cell: ({ row }) => (
      <span className="inline-flex min-w-0 max-w-80 flex-col text-sm">
        <span className="truncate" title={row.original.imageRepository}>{row.original.imageRepository}</span>
        <span className="truncate text-xs text-muted-foreground" title={row.original.tagTemplates.join(', ')}>
          {row.original.tagTemplates.join(', ')}
        </span>
      </span>
    ),
  },
  {
    id: 'lastRun',
    header: ({ column }) => <SortableCell cellName="Last Run" column={column} />,
    cell: ({ row }) => {
      const run = latestRuns.get(row.original.id);
      if (!run) return <span className="text-sm text-muted-foreground">-</span>;

      return (
        <span className="inline-flex items-center gap-2 text-sm">
          <StateIndicator value={run.status} isProcessing={isActiveRun(run)} kind="buildRun" />
          <TimestampCell value={run.completedAt ?? run.startedAt ?? run.queuedAt} formatDateTime={formatDateTime} />
        </span>
      );
    },
    sortingFn: (rowA, rowB) =>
      String(latestRuns.get(rowA.original.id)?.queuedAt ?? '').localeCompare(
        String(latestRuns.get(rowB.original.id)?.queuedAt ?? ''),
      ),
  },
  {
    accessorKey: 'tags',
    header: ({ column }) => <SortableCell cellName="Tags" column={column} />,
    cell: ({ row }) => <TagChips tags={row.original.tags} />,
    sortingFn: (rowA, rowB) =>
      (rowA.original.tags?.map((tag) => tag.name).join(',') ?? '').localeCompare(
        rowB.original.tags?.map((tag) => tag.name).join(',') ?? '',
      ),
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

const BuildNameRow = ({ project, run }: { project: BuildProjectView; run?: BuildRunView }) => (
  <div className="flex min-w-0 items-center gap-1">
    {run ? (
      <StateIndicator value={run.status} isProcessing={isActiveRun(run)} kind="buildRun" />
    ) : (
      <StateIndicator value={project.enabled} enableLabel />
    )}
    <Link to={`../builds/edit/${project.id}`} title={project.name} className="truncate text-sm hover:underline">
      {project.name}
    </Link>
  </div>
);

function indexLatestRuns(runs: BuildRunView[]) {
  const map = new Map<string, BuildRunView>();

  for (const run of runs) {
    const current = map.get(run.buildProjectId);
    if (!current || String(run.queuedAt).localeCompare(String(current.queuedAt)) > 0) {
      map.set(run.buildProjectId, run);
    }
  }

  return map;
}

export function isActiveRun(run: Pick<BuildRunView, 'status'>) {
  return (
    run.status === BuildRunStatus.Queued ||
    run.status === BuildRunStatus.Preparing ||
    run.status === BuildRunStatus.Running
  );
}
