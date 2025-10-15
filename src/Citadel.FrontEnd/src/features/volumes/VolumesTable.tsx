import { DataTable } from '@/components/ui/data-table';
import { DockerVolumeResult } from '@/api/generated/api.types';
import SortableCell from '@/components/ui/SortableCell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useEffect, useCallback, useMemo } from 'react';
import { useAppContext } from '@/AppContext';
import { useVolumesContext } from './VolumesContext';
import DropdownTableMenu from './DropdownTableMenu';
import { DeleteDialog } from './delete-dialog';
import { byteTransform } from '@/lib/bytes.helper';
import { fromNow } from '@/lib/dayjs.helper';
import { VolumeStateIndicator } from './VolumeStateIndicator';
import { useNavigate, useParams } from 'react-router';
import { useRead } from '@/lib/hooks';

export default function VolumesTable() {
  const { currentPlatform } = useAppContext();
  const { data, isLoading, isSuccess } = useRead('listVolumes', { platformId: currentPlatform?.id });
  const { setSelectedRows, setVolumes, volumes, dialogData, setDialogData, requestDelete, deleteIsPending } =
    useVolumesContext();

  // Update volumes when data is fetched
  useEffect(() => {
    if (isSuccess && data?.data.volumes) {
      setVolumes(data.data.volumes);
    }
    return () => {
      setVolumes([]);
      setSelectedRows([]);
    };
  }, [data, isSuccess, setVolumes, setSelectedRows]);

  // Memoized selection change handler
  const handleSelectionChange = useCallback(
    (ids: string[]) => {
      setSelectedRows(volumes?.filter((c) => ids.includes(c.id!)));
    },
    [volumes, setSelectedRows],
  );

  // Memoized row count
  const rowCount = useMemo(() => volumes?.length ?? 0, [volumes]);

  return (
    <>
      <div className="flex flex-col gap-3">
        <DataTable
          columns={columns}
          data={volumes ?? []}
          isLoading={isLoading}
          onSelectionChange={handleSelectionChange}
        />
        <div className="text-muted-foreground text-xs p-2 font-normal">
          {rowCount > 0 && (
            <span>
              Showing {rowCount} of {rowCount} volume(s)
            </span>
          )}
        </div>
      </div>
      <DeleteDialog
        dialogData={dialogData}
        setDialogData={setDialogData}
        requestDelete={requestDelete}
        deleteIsPending={deleteIsPending}
      />
    </>
  );
}

const columns: ColumnDef<DockerVolumeResult>[] = [
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
        aria-label="Select volume"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <VolumeNameRow volume={row.original} />,
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.name?.localeCompare(rowB.original?.name),
  },
  {
    accessorKey: 'created',
    header: ({ column }) => <SortableCell cellName="Created" column={column} />,
    cell: ({ row }) => <span className="">{fromNow(new Date(row.original.createdAt as any).getTime())}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
  },
  {
    accessorKey: 'driver',
    header: ({ column }) => <SortableCell cellName="Driver" column={column} />,
    cell: ({ row }) => <span className="">{row.original.driver}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
  },
  {
    accessorKey: 'scope',
    header: ({ column }) => <SortableCell cellName="Scope" column={column} />,
    cell: ({ row }) => <span className="">{row.original.scope}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
  },
  {
    accessorKey: 'size',
    header: ({ column }) => <SortableCell cellName="Size" column={column} />,
    cell: ({ row }) => <span className="">{byteTransform(row.original.usageData?.size, 2)}</span>,
    sortingFn: (rowA, rowB) => {
      const sizeA = rowA.original.usageData?.size ?? 0;
      const sizeB = rowB.original.usageData?.size ?? 0;
      return sizeA < sizeB ? 1 : -1;
    },
  },
  {
    id: 'actions',
    cell: ({ row }) => (
      <div className="text-center">
        <DropdownTableMenu volume={row.original} />
      </div>
    ),
  },
];

const VolumeNameRow = ({ volume }: { volume: DockerVolumeResult }) => {
  const { platformId } = useParams<{ platformId: string }>();
  const navigate = useNavigate();
  function onClick() {
    navigate(`/platforms/${platformId}/volumes/${volume.id}/`);
  }
  return (
    <div className="flex items-center whitespace-nowrap">
      <div className="flex items-center">
        <VolumeStateIndicator inUse={volume.inUse ?? false} />
      </div>
      <span
        className="cursor-pointer hover:underline text-[13px]"
        onClick={onClick}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            onClick();
          }
        }}
        tabIndex={0}
        role="button"
        aria-label="Show volume details">
        {volume.id}
      </span>
    </div>
  );
};
