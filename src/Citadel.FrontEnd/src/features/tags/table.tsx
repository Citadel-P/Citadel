import { TagView } from '@/api/generated/api.types';
import { ContentCard } from '@/components/custom/content-card';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import SortableCell from '@/components/custom/sortable-cell';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { fromNow } from '@/lib/dayjs.helper';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { ColumnDef } from '@tanstack/react-table';
import { useEffect, useMemo, useState } from 'react';
import { subscribeTagPageActions, TagPageAction } from './actions';
import { TagFormDialog } from './dialogs';
import { getTagColorLabel, getTagTextColor } from './tag-colors';

export function TagsTable({
  items,
  actions,
  isLoading,
}: {
  items: TagView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: TagView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) {
  const [_, setSelectedResources] = useSelectedResources<TagView>('Tag');
  const [editingTag, setEditingTag] = useState<TagView | null>(null);
  const cols = useMemo(() => columns(actions ?? {}), [actions]);

  useEffect(() => {
    return subscribeTagPageActions((action: TagPageAction) => {
      if (action.type === 'edit-tag') setEditingTag(action.tag);
    });
  }, []);

  return (
    <ContentCard>
      <DataTable columns={cols} data={items} isLoading={isLoading} onSelectionChange={setSelectedResources} />
      <TagFormDialog
        mode="edit"
        tag={editingTag ?? undefined}
        open={editingTag !== null}
        onOpenChange={(open) => !open && setEditingTag(null)}
      />
    </ContentCard>
  );
}

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: TagView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
): ColumnDef<TagView>[] => [
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
        aria-label="Select tag"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <TagName tag={row.original} />,
    sortingFn: (rowA, rowB) => rowA.original.name.localeCompare(rowB.original.name),
  },
  {
    accessorKey: 'color',
    header: ({ column }) => <SortableCell cellName="Color Name" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{getTagColorLabel(row.original.color)}</span>,
    sortingFn: (rowA, rowB) =>
      getTagColorLabel(rowA.original.color).localeCompare(getTagColorLabel(rowB.original.color)),
  },
  {
    accessorKey: 'usageCount',
    header: ({ column }) => <SortableCell cellName="Usage" column={column} />,
    cell: ({ row }) => <span className="text-[13px] tabular-nums">{row.original.usageCount}</span>,
    sortingFn: (rowA, rowB) => Number(rowA.original.usageCount) - Number(rowB.original.usageCount),
  },
  {
    accessorKey: 'updatedAt',
    header: ({ column }) => <SortableCell cellName="Updated" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{fromNow(row.original.updatedAt)}</span>,
    sortingFn: (rowA, rowB) => String(rowA.original.updatedAt).localeCompare(String(rowB.original.updatedAt)),
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

function TagName({ tag }: { tag: TagView }) {
  const textColor = getTagTextColor(tag.color);

  return (
    <div className="flex min-w-0 items-center">
      <div
        className="max-w-64 truncate rounded-md  border-none px-2 py-1 text-sm font-medium"
        style={{ backgroundColor: tag.color, color: textColor }}>
        {tag.name}
      </div>
    </div>
  );
}
