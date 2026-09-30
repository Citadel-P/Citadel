import { DataTable } from '@/components/ui/data-table';
import { RegistryStatus, AuthorizedRegistryView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { ColumnDef, Row } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { Link } from 'react-router';
import { Globe, InfoIcon } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { ActionData } from '@/pages/types';
import { useSelectedResources } from '@/lib/atoms';
import { useMemo } from 'react';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { DockerIcon, GitHubIcon } from '@/lib/icons';
import { StateIndicator } from '@/components/custom/state-indicator';
import { TagChips } from '@/features/tags/components';

const getNonDefaultRows = (rows: Row<AuthorizedRegistryView>[]) => rows.filter((row) => !row.original.isDefault);

const areAllNonDefaultRowsSelected = (rows: Row<AuthorizedRegistryView>[]) =>
  getNonDefaultRows(rows).every((row) => row.getIsSelected());

const areSomeNonDefaultRowsSelected = (rows: Row<AuthorizedRegistryView>[]) =>
  getNonDefaultRows(rows).some((row) => row.getIsSelected());

const providerIcons: Record<string, React.FC<{ className?: string }>> = {
  Custom: Globe,
  DockerHub: DockerIcon,
  GitHub: GitHubIcon,
};

const RenderProvider = ({ registry }: { registry: AuthorizedRegistryView }) => {
  const Icon = providerIcons[registry.type];

  return (
    <div className="flex flex-row gap-1 items-center">
      {Icon && <Icon className="h-4 w-4 text-foreground/50" />}
      {registry.type}
    </div>
  );
};

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: AuthorizedRegistryView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
): ColumnDef<AuthorizedRegistryView>[] => [
  {
    id: 'select',
    header: ({ table }) => {
      const nonDefaultRows = getNonDefaultRows(table.getRowModel().rows);
      const allNonDefaultRowsSelected = areAllNonDefaultRowsSelected(table.getRowModel().rows);
      const someNonDefaultRowsSelected = areSomeNonDefaultRowsSelected(table.getRowModel().rows);

      return (
        <Checkbox
          checked={allNonDefaultRowsSelected || (someNonDefaultRowsSelected && 'indeterminate')}
          onCheckedChange={(value) => {
            nonDefaultRows.forEach((row) => row.toggleSelected(!!value));
          }}
          aria-label="Select all"
        />
      );
    },
    cell: ({ row }) => {
      if (row.original.isDefault) {
        return <></>;
      }
      return (
        <Checkbox
          checked={row.getIsSelected()}
          onCheckedChange={(value) => row.toggleSelected(!!value)}
          aria-label="Select Registry"
        />
      );
    },

    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <RegistryNameRow registry={row.original} />,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => rowA.original.name.localeCompare(rowB.original.name),
  },
  {
    accessorKey: 'provider',
    header: ({ column }) => <SortableCell cellName="Provider" column={column} />,
    cell: ({ row }) => <RenderProvider registry={row.original} />,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.type.localeCompare(rowB.original.type);
    },
  },
  {
    accessorKey: 'registryHost',
    header: ({ column }) => <SortableCell cellName="Host" column={column} />,
    cell: ({ row }) => <div>{row.original.registryHost}</div>,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.registryHost.localeCompare(rowB.original.registryHost);
    },
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
    cell: ({ row }) => {
      if (row.original.isDefault) {
        return <></>;
      }
      return <RowActionMenu resource={row.original} actions={actions} />;
    },
  },
];

export const RegistriesTable = ({
  items,
  actions,
  isLoading,
}: {
  items: AuthorizedRegistryView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: AuthorizedRegistryView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [_, setSelectedResources] = useSelectedResources<AuthorizedRegistryView>('Registry');
  const cols = useMemo(() => columns(actions ?? {}), [actions]);

  return <DataTable columns={cols} data={items} isLoading={isLoading} onSelectionChange={setSelectedResources} />;
};

const RegistryNameRow = ({ registry }: { registry: AuthorizedRegistryView }) => {
  const isDefault = registry.isDefault;
  const status = isDefault ? RegistryStatus.Active : (registry.status ?? RegistryStatus.Disabled);

  const name = isDefault ? (
    <span>{registry.name}</span>
  ) : (
    <Link to={`../registries/edit/${registry.id}`} className="hover:underline">
      {registry.name}
    </Link>
  );

  return (
    <div className="flex items-center gap-1">
      <StateIndicator value={status} />
      {name}
      {isDefault && (
        <TooltipProvider delayDuration={200}>
          <Tooltip>
            <TooltipTrigger asChild>
              <InfoIcon className="h-3.5 w-3.5 text-foreground/65" />
            </TooltipTrigger>
            <TooltipContent>
              <p>This is the default registry, it can&apos;t be deleted or updated.</p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      )}
    </div>
  );
};
