import { ResourceBindingKind, ResourceBindingView, SecretDefinitionView } from '@/api/generated/api.types';
import { ContentCard } from '@/components/custom/content-card';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import SortableCell from '@/components/custom/sortable-cell';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { ColumnDef } from '@tanstack/react-table';
import { KeyRound, Variable } from 'lucide-react';
import { useMemo } from 'react';

const EMPTY_ENTRIES: ResourceBindingView[] = [];
const ENV_DELIVERY_MODE = 'EnvironmentVariable';
const MOUNTED_FILE_DELIVERY_MODE = 'MountedFile';

export const ResourceBindingsDataTable = ({
  items,
  secrets,
  isLoading,
  actions,
}: {
  items: ResourceBindingView[];
  secrets: SecretDefinitionView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: ResourceBindingView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [, setSelectedResources] = useSelectedResources<ResourceBindingView>('Binding');
  const secretNames = useMemo(() => new Map(secrets.map((secret) => [secret.id, secret.name])), [secrets]);
  const cols = useMemo(() => columns({ secretNames, actions }), [actions, secretNames]);

  return (
    <ContentCard>
      <DataTable
        columns={cols}
        data={items ?? EMPTY_ENTRIES}
        isLoading={isLoading}
        onSelectionChange={setSelectedResources}
      />
    </ContentCard>
  );
};

const columns = ({
  secretNames,
  actions,
}: {
  secretNames: Map<string, string>;
  actions: Record<
    string,
    React.FC<{ resource: ResourceBindingView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}): ColumnDef<ResourceBindingView>[] => [
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
        aria-label="Select binding"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <NameCell entry={row.original} />,
    sortingFn: (rowA: any, rowB: any): number => rowA.original.name.localeCompare(rowB.original.name),
  },
  {
    accessorKey: 'kind',
    header: ({ column }) => <SortableCell cellName="Type" column={column} />,
    cell: ({ row }) => <KindBadge kind={row.original.kind} />,
    sortingFn: (rowA: any, rowB: any): number => rowA.original.kind.localeCompare(rowB.original.kind),
  },
  {
    accessorKey: 'value',
    header: ({ column }) => <SortableCell cellName="Value" column={column} />,
    cell: ({ row }) => <ValueCell entry={row.original} secretNames={secretNames} />,
  },
  {
    accessorKey: 'secretDeliveryMode',
    header: ({ column }) => <SortableCell cellName="Delivery" column={column} />,
    cell: ({ row }) => <DeliveryCell entry={row.original} />,
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

const NameCell = ({ entry }: { entry: ResourceBindingView }) => (
  <div className="flex items-center gap-2 py-2">
    {entry.kind === ResourceBindingKind.Secret ? (
      <KeyRound className="h-3.5 w-3.5 text-muted-foreground" />
    ) : (
      <Variable className="h-3.5 w-3.5 text-muted-foreground" />
    )}
    <span className="font-mono text-xs">{entry.name}</span>
  </div>
);

const KindBadge = ({ kind }: { kind: ResourceBindingKind }) => (
  <Badge
    variant="outline"
    className={
      `gap-1 border-0 p-1 px-2 ` +
      (kind === ResourceBindingKind.Secret ? 'bg-amber-200/25 text-amber-500' : 'bg-green-200/25 text-green-500')
    }>
    {kind === ResourceBindingKind.Secret ? 'Secret' : 'Variable'}
  </Badge>
);

const ValueCell = ({ entry, secretNames }: { entry: ResourceBindingView; secretNames: Map<string, string> }) => {
  if (entry.kind === ResourceBindingKind.Secret) {
    const secretName = entry.secretId ? secretNames.get(entry.secretId) : undefined;
    return (
      <div className="flex flex-col gap-0.5 py-2">
        <span className="font-mono text-xs">{secretName ?? 'Unbound secret'}</span>
        <span className="font-mono text-xs text-muted-foreground">********</span>
      </div>
    );
  }

  return <span className="font-mono text-xs">{entry.value}</span>;
};

const DeliveryCell = ({ entry }: { entry: ResourceBindingView }) => {
  if (entry.kind !== ResourceBindingKind.Secret) {
    return <span className="text-xs text-muted-foreground">-</span>;
  }

  if (entry.secretDeliveryMode === MOUNTED_FILE_DELIVERY_MODE) {
    return (
      <div className="flex flex-col gap-0.5 text-xs text-muted-foreground">
        <span>Mounted file</span>
        {entry.targetPath && <span className="font-mono">{entry.targetPath}</span>}
      </div>
    );
  }

  return <span className="text-xs text-muted-foreground">{entry.secretDeliveryMode ?? ENV_DELIVERY_MODE}</span>;
};
