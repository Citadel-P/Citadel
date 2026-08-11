import { DataTable } from '@/components/ui/data-table';
import { ImageView, PlatformType, ResourceControlState } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { truncate } from '@/lib/truncate';
import { useMemo } from 'react';
import { byteTransform } from '@/lib/bytes.helper';
import { useNavigate, useParams } from 'react-router';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import { formatId } from '@/lib/utils';
import { RegistryDisplay } from '@/components/custom/registry-display';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { useAppContext } from '@/lib/context/app-context';
import { createNodeResourceColumn } from '@/components/custom/node-resource-column';

export const ImagesTable = ({
  items,
  isLoading,
  actions,
}: {
  items: ImageView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: ImageView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [_, setSelectedResources] = useSelectedResources<ImageView>('Image');
  const { currentPlatform } = useAppContext();
  const formatDateTime = useProfileDateTimeFormatter();
  const showNode = currentPlatform?.type === PlatformType.DockerSwarm;
  const cols = useMemo(() => columns(actions ?? {}, formatDateTime, showNode), [actions, formatDateTime, showNode]);

  return <DataTable columns={cols} data={items} isLoading={isLoading} onSelectionChange={setSelectedResources} />;
};

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: ImageView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
  formatDateTime: DateTimeFormatter,
  showNode: boolean,
): ColumnDef<ImageView>[] => [
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
        aria-label="Select image"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <ImageNameRow image={row.original} />,
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.name?.localeCompare(rowB.original?.name),
  },
  ...(showNode ? [createNodeResourceColumn<ImageView>()] : []),
  {
    accessorKey: 'tags',
    header: ({ column }) => <SortableCell cellName="Tags" column={column} />,
    cell: ({ row }) => (
      <div className="flex flex-wrap gap-1">
        {row.original.tags.map((t) => (
          <span key={t} className="px-2 py-0.5 bg-muted/25 rounded text-sm" title={t}>
            {truncate(t ?? '', 24)}
          </span>
        ))}
      </div>
    ),
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.tag?.localeCompare(rowB.original?.tag),
  },
  {
    accessorKey: 'registry',
    header: ({ column }) => <SortableCell cellName="Registry" column={column} />,
    cell: ({ row }) => <RegistryDisplay registry={row.original.registry ?? undefined} />,
    sortingFn: (rowA: any, rowB: any): number =>
      rowA.original.registry?.name?.localeCompare(rowB.original.registry?.name),
  },
  {
    accessorKey: 'id',
    header: ({ column }) => <SortableCell cellName="Image Id" column={column} />,
    cell: ({ row }) => (
      <CopyToClipboard
        textToCopy={row.original.dockerImageId}
        transform={formatId}
        groupClassName="rowid"
        textClassName=""
      />
    ),
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.imageId?.localeCompare(rowB.original?.imageId),
  },
  {
    accessorKey: 'created',
    header: ({ column }) => <SortableCell cellName="Created" column={column} />,
    cell: ({ row }) => <TimestampCell value={row.original.createdAt} formatDateTime={formatDateTime} />,
    sortingFn: (rowA, rowB) => (rowA.original.createdAt < rowB.original.createdAt ? 1 : -1),
  },
  {
    accessorKey: 'size',
    header: ({ column }) => <SortableCell cellName="Size" column={column} />,
    cell: ({ row }) => <span className="">{byteTransform(row.original.size, 2)}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.size < rowB.original.size ? 1 : -1),
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

const ImageNameRow = ({ image }: { image: ImageView }) => {
  const { platformId } = useParams<{ platformId: string }>();
  const navigate = useNavigate();
  function onNameClick() {
    const nodeQuery = image.dockerNodeId ? `?dockerNodeId=${encodeURIComponent(image.dockerNodeId)}` : '';
    navigate(`/platforms/${platformId}/images/${formatId(image.dockerImageId)}/${nodeQuery}`);
  }
  return (
    <div className="flex items-center whitespace-nowrap">
      <div className="flex items-center">
        <StateIndicator value={image.isInUse} isProcessing={image.controlState === ResourceControlState.Processing} />
      </div>
      <span
        className="cursor-pointer table-link"
        onClick={onNameClick}
        title={image.name}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            onNameClick();
          }
        }}
        tabIndex={0}
        role="button"
        aria-label="Show image details">
        {truncate(image.name?.length > 0 ? image.name : '<none>', 32, 'right')}
      </span>
    </div>
  );
};
