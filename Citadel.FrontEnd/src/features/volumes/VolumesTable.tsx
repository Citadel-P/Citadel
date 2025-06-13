import { DataTable } from '@/components/ui/data-table';
import { DockerVolume } from '@/api/_generated';
import SortableCell from '@/components/ui/SortableCell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useContextSelector } from 'use-context-selector';
import { useEffect, useCallback, useMemo, memo } from 'react';
import { AppContext } from '@/AppProvider';
import { useGETVolumes } from './hooks/useGETVolumes';
import { VolumesContext } from './VolumesProvider';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import DropdownTableMenu from './DropdownTableMenu';
import { VolumeInspectSheet } from './VolumeInspectSheet';
import { DeleteVolumeDialog } from './dialogs/DeleteVolumeDialog';
import { byteTransform } from '@/lib/bytes.helper';
import { fromNow } from '@/lib/dayjs.helper';

export default function VolumesTable() {
  const currentPlatform = useContextSelector(AppContext, (v) => v?.currentPlatform);
  const { data, isLoading, isSuccess } = useGETVolumes(currentPlatform?.id);
  const setSelectedRows = useContextSelector(VolumesContext, (v) => v?.setSelectedRows)!;
  const setVolumes = useContextSelector(VolumesContext, (v) => v?.setVolumes)!;
  const volumes = useContextSelector(VolumesContext, (v) => v?.volumes);
  const setSheetOpen = useContextSelector(VolumesContext, (v) => v?.setSheetOpen)!;
  const setCurrentVolume = useContextSelector(VolumesContext, (v) => v?.setCurrentVolume)!;
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

  const handleShowSheet = (volume: DockerVolume) => {
    setCurrentVolume(volume);
    setSheetOpen(true);
  };

  return (
    <>
      <div className="flex flex-col gap-3">
        <DataTable
          columns={columns(handleShowSheet)}
          data={volumes ?? []}
          isLoading={isLoading}
          onSelectionChange={handleSelectionChange}
        />
        <div className="text-muted-foreground text-xs font-normal">
          {rowCount > 0 && (
            <span>
              Showing {rowCount} of {rowCount} volume(s)
            </span>
          )}
        </div>
      </div>
      <DeleteVolumeDialog />
      <VolumeInspectSheet />
    </>
  );
}

const columns = (handleShowSheet: (volume: DockerVolume) => void): ColumnDef<DockerVolume>[] => [
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
    cell: ({ row }) => <VolumeNameRow volume={row.original} onShowSheet={handleShowSheet} />,
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.name?.localeCompare(rowB.original?.name),
  },
  {
    accessorKey: 'created',
    header: ({ column }) => <SortableCell cellName="Created" column={column} />,
    cell: ({ row }) => (
      <span className="text-[13px]">{fromNow(new Date(row.original.createdAt as any).getTime())}</span>
    ),
    sortingFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
  },
  {
    accessorKey: 'size',
    header: ({ column }) => <SortableCell cellName="Size" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{byteTransform(row.original.usageData.size, 2)}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.usageData.size! < rowB.original.usageData.size! ? 1 : -1),
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

const VolumeNameRow = ({
  volume,
  onShowSheet,
}: {
  volume: DockerVolume;
  onShowSheet: (volume: DockerVolume) => void;
}) => {
  return (
    <div className="flex items-center whitespace-nowrap">
      <div className="flex items-center">
        <VolumeStatusTooltip inUse={volume.inUse ?? false} />
      </div>
      <span
        className="cursor-pointer hover:underline"
        onClick={() => onShowSheet(volume)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            onShowSheet(volume);
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

const VolumeStatusTooltip = memo(({ inUse }: { inUse: boolean }) => {
  const getStatusClass = () => (inUse ? 'bg-green-500' : 'bg-gray-500');

  return (
    <TooltipProvider delayDuration={200}>
      <Tooltip>
        <TooltipTrigger asChild>
          <div className={`${getStatusClass()} mr-2 h-2 w-2 rounded-full`} />
        </TooltipTrigger>
        <TooltipContent>
          <span>{inUse ? 'In use' : 'Unused'}</span>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
});

VolumeStatusTooltip.displayName = 'VolumeStatusTooltip';
