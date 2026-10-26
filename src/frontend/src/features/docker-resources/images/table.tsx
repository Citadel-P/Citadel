import { ImageReferenceDialog, type ImageReferenceAction } from './reference-actions';
import { ImageUpdateStatus } from './update-status';
import { ImageUsedBy } from './used-by';
import { DataTable } from '@/components/ui/data-table';
import { ImageView, PlatformType, ResourceControlState } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { StockFeatures, ColumnDef, type CellContext } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { truncate } from '@/lib/truncate';
import { useMemo, useState } from 'react';
import { byteTransform } from '@/lib/bytes.helper';
import { Link, useParams } from 'react-router';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import { formatId } from '@/lib/utils';
import { RegistryDisplay } from '@/components/custom/registry-display';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { Timestamp } from '@/components/custom/timestamp';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { useAppContext } from '@/lib/context/app-context';
import { createNodeResourceColumn } from '@/components/custom/node-resource-column';
import { useAtomValue } from 'jotai';
import { imageShowRegistryAtom, imageShowCreatedAtom, imageShowIdAtom } from './table-columns';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';

export const ImagesTable = ({
  items,
  isLoading,
  isFiltered,
  actions,
}: {
  items: ImageView[];
  isLoading: boolean;
  isFiltered?: boolean;
  actions: Record<
    string,
    React.FC<{ resource: ImageView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [_, setSelectedResources] = useSelectedResources<ImageView>('Image');
  const { currentPlatform } = useAppContext();
  const formatDateTime = useProfileDateTimeFormatter();
  const [referenceAction, setReferenceAction] = useState<{ image: ImageView; kind: ImageReferenceAction } | null>(null);
  const showRegistry = useAtomValue(imageShowRegistryAtom);
  const showCreated = useAtomValue(imageShowCreatedAtom);
  const showId = useAtomValue(imageShowIdAtom);
  const showNode = currentPlatform?.type === PlatformType.DockerSwarm;
  // Visibility changes must retain cell component identities and their mounted query observers.
  const allColumns = useMemo(
    () => columns(actions ?? {}, formatDateTime, showNode, (image, kind) => setReferenceAction({ image, kind })),
    [actions, formatDateTime, showNode],
  );
  const cols = useMemo(
    () =>
      allColumns.filter(
        (column) =>
          (column.id !== 'registry' || showRegistry) &&
          (column.id !== 'created' || showCreated) &&
          (column.id !== 'dockerImageId' || showId),
      ),
    [allColumns, showRegistry, showCreated, showId],
  );

  return (
    <>
      <DataTable
        columns={cols}
        data={items}
        isLoading={isLoading}
        onSelectionChange={setSelectedResources}
        emptyState={
          isFiltered
            ? {
                title: 'No images match the current filters.',
                description: 'Adjust the search or turn off Updates available.',
              }
            : undefined
        }
      />
      {referenceAction && (
        <ImageReferenceDialog
          image={referenceAction.image}
          kind={referenceAction.kind}
          onClose={() => setReferenceAction(null)}
        />
      )}
    </>
  );
};

const ImageUpdateStatusCell = ({ row }: CellContext<StockFeatures, ImageView, unknown>) => (
  <ImageUpdateStatus updates={row.original.updates} />
);

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: ImageView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
  formatDateTime: DateTimeFormatter,
  showNode: boolean,
  onReference: (image: ImageView, kind: ImageReferenceAction) => void,
): ColumnDef<StockFeatures, ImageView>[] => [
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
    header: ({ column }) => <SortableCell cellName="Image" column={column} />,
    cell: ({ row }) => <ImageNameRow image={row.original} />,
    sortFn: (rowA: any, rowB: any): number => rowA.original?.name?.localeCompare(rowB.original?.name),
  },
  ...(showNode ? [createNodeResourceColumn<ImageView>()] : []),
  {
    id: 'dockerImageId',
    accessorKey: 'dockerImageId',
    header: ({ column }) => <SortableCell cellName="ID" column={column} />,
    cell: ({ row }) => (
      <CopyToClipboard
        textToCopy={row.original.dockerImageId}
        transform={formatId}
        groupClassName="rowid"
        textClassName="text-sm text-muted-foreground font-mono"
      />
    ),
  },
  {
    id: 'updateStatus',
    header: () => <span>Update status</span>,
    cell: ImageUpdateStatusCell,
  },
  {
    id: 'usedBy',
    header: () => <span>Used by</span>,
    cell: ({ row }) => <ImageUsedBy image={row.original} />,
  },
  {
    id: 'registry',
    accessorKey: 'registry',
    header: ({ column }) => <SortableCell cellName="Registry" column={column} />,
    cell: ({ row }) => <RegistryDisplay registry={row.original.registry ?? undefined} />,
    sortFn: (rowA: any, rowB: any): number => rowA.original.registry?.name?.localeCompare(rowB.original.registry?.name),
  },
  {
    id: 'created',
    accessorKey: 'createdAt',
    header: ({ column }) => <SortableCell cellName="Created" column={column} />,
    cell: ({ row }) => <Timestamp value={row.original.createdAt} formatDateTime={formatDateTime} />,
    sortFn: (rowA, rowB) => (rowA.original.createdAt < rowB.original.createdAt ? 1 : -1),
  },
  {
    accessorKey: 'size',
    header: ({ column }) => <SortableCell cellName="Size" column={column} />,
    cell: ({ row }) => <span className="">{byteTransform(row.original.size, 2)}</span>,
    sortFn: (rowA, rowB) => (rowA.original.size < rowB.original.size ? 1 : -1),
  },
  {
    id: 'actions',
    cell: ({ row }) => (
      <RowActionMenu
        resource={row.original}
        actions={actions}
        onAction={(action) => {
          if (action.key === 'checkImage') onReference(row.original, 'check');
          if (action.key === 'pullImage') onReference(row.original, 'pull');
        }}
      />
    ),
  },
];

const ImageNameRow = ({ image }: { image: ImageView }) => {
  const { platformId } = useParams<{ platformId: string }>();
  const tags = [...new Set(image.tags.filter((tag) => tag && !tag.includes('<')))];
  const reference = tags[0];
  const tagSeparator = reference && !reference.includes('@') ? reference.lastIndexOf(':') : -1;
  const hasTag = !!reference && tagSeparator > reference.lastIndexOf('/');
  const name = reference ? (hasTag ? reference.slice(0, tagSeparator) : reference) : image.name || '<none>';
  const tag = hasTag ? reference.slice(tagSeparator + 1) : undefined;
  const nodeQuery = image.dockerNodeId ? `?dockerNodeId=${encodeURIComponent(image.dockerNodeId)}` : '';
  return (
    <div className="min-w-0 space-y-0.5">
      <div className="flex items-center gap-1 whitespace-nowrap">
        <StateIndicator value={image.isInUse} isProcessing={image.controlState === ResourceControlState.Processing} />
        <Link
          className="table-link min-w-0 truncate max-w-64"
          to={`/platforms/${platformId}/images/${formatId(image.dockerImageId)}/${nodeQuery}`}
          title={name}
          aria-label={`Show image details: ${name}`}>
          {name}
        </Link>
        {tag && (
          <Badge
            variant="outline"
            className="border-transparent bg-muted/50 px-1.5 py-0 font-normal text-muted-foreground"
            title={tag}>
            {truncate(tag, 20)}
          </Badge>
        )}
        {!reference && <span className="text-xs text-muted-foreground">Untagged</span>}
        {tags.length > 1 && (
          <Popover>
            <PopoverTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 px-1.5 text-xs font-normal text-muted-foreground"
                aria-label={`Show all ${tags.length} image tags`}>
                +{tags.length - 1}
              </Button>
            </PopoverTrigger>
            <PopoverContent align="start" className="w-80">
              <p className="mb-2 text-sm font-medium">Image tags</p>
              <ul className="max-h-60 space-y-1 overflow-y-auto text-sm">
                {tags.map((value) => (
                  <li key={value} className="break-all">
                    {value}
                  </li>
                ))}
              </ul>
            </PopoverContent>
          </Popover>
        )}
      </div>
    </div>
  );
};
