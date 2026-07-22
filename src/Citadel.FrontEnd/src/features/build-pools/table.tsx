import {
  BuildAgentPoolProvider,
  BuildAgentPoolConnectionMode,
  BuildAgentPoolProviderSpecAwsEc2BuildAgentPoolProviderSpec,
  BuildAgentPoolProviderSpecSelfManagedVmBuildAgentPoolProviderSpec,
  BuildAgentPoolValidationStatus,
  BuildAgentPoolView,
  ResourceControlState,
} from '@/api/generated/api.types';
import { ContentCard } from '@/components/custom/content-card';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { TagChips } from '@/features/tags/components';
import { useSelectedResources } from '@/lib/atoms';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { ActionData } from '@/pages/types';
import { ColumnDef } from '@tanstack/react-table';
import { Cpu, ShieldCheck, Server } from 'lucide-react';
import { useMemo } from 'react';
import { Link } from 'react-router';

type ActionMap = Record<
  string,
  React.FC<{ resource: BuildAgentPoolView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
>;

export function BuildPoolsTable({
  items,
  actions,
  isLoading,
}: {
  items: BuildAgentPoolView[];
  isLoading: boolean;
  actions: ActionMap;
}) {
  const [, setSelectedResources] = useSelectedResources<BuildAgentPoolView>('BuildAgentPool');
  const formatDateTime = useProfileDateTimeFormatter();
  const cols = useMemo(() => columns(actions ?? {}, formatDateTime), [actions, formatDateTime]);

  return (
    <ContentCard>
      <DataTable columns={cols} data={items} isLoading={isLoading} onSelectionChange={setSelectedResources} />
    </ContentCard>
  );
}

const columns = (actions: ActionMap, formatDateTime: DateTimeFormatter): ColumnDef<BuildAgentPoolView>[] => [
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
        aria-label="Select build pool"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <BuildPoolNameRow pool={row.original} />,
    sortingFn: (rowA, rowB) => rowA.original.name.localeCompare(rowB.original.name),
  },
  {
    accessorKey: 'provider',
    header: ({ column }) => <SortableCell cellName="Provider" column={column} />,
    cell: ({ row }) => <ProviderCell pool={row.original} />,
    sortingFn: (rowA, rowB) => rowA.original.provider.localeCompare(rowB.original.provider),
  },
  {
    accessorKey: 'capacity',
    header: ({ column }) => <SortableCell cellName="Capacity" column={column} />,
    cell: ({ row }) => (
      <span className="inline-flex items-center gap-2 text-sm">
        <Cpu className="size-3.5 text-muted-foreground" />
        {row.original.maxActiveBuilders} active
      </span>
    ),
    sortingFn: (rowA, rowB) => Number(rowA.original.maxActiveBuilders) - Number(rowB.original.maxActiveBuilders),
  },
  {
    accessorKey: 'lastValidationStatus',
    header: ({ column }) => <SortableCell cellName="Validation" column={column} />,
    cell: ({ row }) => <ValidationCell pool={row.original} formatDateTime={formatDateTime} />,
    sortingFn: (rowA, rowB) => rowA.original.lastValidationStatus.localeCompare(rowB.original.lastValidationStatus),
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

const BuildPoolNameRow = ({ pool }: { pool: BuildAgentPoolView }) => (
  <div className="flex min-w-0 items-center gap-1">
    <StateIndicator value={pool.enabled} enableLabel />
    <Link to={`../build-pools/edit/${pool.id}`} title={pool.name} className="truncate text-sm hover:underline">
      {pool.name}
    </Link>
  </div>
);

const ProviderCell = ({ pool }: { pool: BuildAgentPoolView }) => {
  const aws = pool.provider === BuildAgentPoolProvider.AwsEc2
    ? (pool.providerSpec as BuildAgentPoolProviderSpecAwsEc2BuildAgentPoolProviderSpec)
    : undefined;
  const vm = pool.provider === BuildAgentPoolProvider.SelfManagedVm
    ? (pool.providerSpec as BuildAgentPoolProviderSpecSelfManagedVmBuildAgentPoolProviderSpec)
    : undefined;
  const label = aws ? 'AWS EC2' : vm ? 'Self-managed VM' : pool.provider;
  const vmTarget = vm?.connectionMode === BuildAgentPoolConnectionMode.EdgeAgent
    ? 'Edge Agent'
    : vm?.endpoint;
  const details = aws ? `${aws.region} - ${aws.instanceType}` : vm ? `${vmTarget ?? 'Unconfigured'} - ${vm.maxWorkers} worker(s)` : '';

  return (
    <span className="inline-flex min-w-0 max-w-80 items-center gap-2 text-sm">
      <Server className="size-3.5 shrink-0 text-muted-foreground" />
      <span className="truncate" title={details || label}>
        {label}
        {details ? ` - ${details}` : ''}
      </span>
    </span>
  );
};

const ValidationCell = ({
  pool,
  formatDateTime,
}: {
  pool: BuildAgentPoolView;
  formatDateTime: DateTimeFormatter;
}) => {
  const testedAt = pool.lastValidatedAt ? formatDateTime(pool.lastValidatedAt) : 'Not tested yet';
  const title = [validationLabel(pool.lastValidationStatus), testedAt, pool.lastValidationMessage].filter(Boolean).join(' - ');

  return (
    <span className="inline-flex min-w-0 max-w-72 items-center gap-2 text-sm" title={title}>
      <StateIndicator
        value={pool.lastValidationStatus}
        kind="buildAgentPoolValidation"
        isProcessing={pool.controlState === ResourceControlState.Processing}
      />
      <ShieldCheck className="size-3.5 shrink-0 text-muted-foreground" />
      <span className="min-w-0 truncate">
        <span>{validationLabel(pool.lastValidationStatus)}</span>
        <span className="ml-2 text-xs text-muted-foreground">{testedAt}</span>
      </span>
    </span>
  );
};

function validationLabel(status: BuildAgentPoolValidationStatus) {
  switch (status) {
    case BuildAgentPoolValidationStatus.NotTested:
      return 'Not tested';
    case BuildAgentPoolValidationStatus.Ready:
      return 'Ready';
    case BuildAgentPoolValidationStatus.Invalid:
      return 'Invalid';
    case BuildAgentPoolValidationStatus.Degraded:
      return 'Degraded';
    default:
      return String(status);
  }
}
