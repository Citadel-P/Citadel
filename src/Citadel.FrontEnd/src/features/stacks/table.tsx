import { DataTable } from '@/components/ui/data-table';
import { StackView, ResourceControlState } from '@/api/generated/api.types';
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
import { PlatformStatusCell } from '@/components/custom/common';
import { truncate } from '@/lib/truncate';

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
