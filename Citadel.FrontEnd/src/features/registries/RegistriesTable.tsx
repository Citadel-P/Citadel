import { DataTable } from '@/components/ui/data-table';
import { RegistryView } from '@/api/_generated';
import SortableCell from '@/components/ui/SortableCell';
import DropdownTableMenu from './DropdownTableMenu';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useContextSelector } from 'use-context-selector';
import { RegistriesContext } from './RegistriesProvider';
import { Link } from 'react-router';

const columns: ColumnDef<RegistryView>[] = [
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
        aria-label="Select registry"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => (
      <Link to={`../registries/edit/${row.original.id}`} className="hover:underline">
        {row.original.name}
      </Link>
    ),
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.name < rowB.original.name ? 1 : -1;
    },
  },
  {
    accessorKey: 'provider',
    header: ({ column }) => <SortableCell cellName="Provider" column={column} />,
    cell: ({ row }) => <div>{row.original.discriminator}</div>,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.discriminator < rowB.original.discriminator ? 1 : -1;
    },
  },
  {
    accessorKey: 'url',
    header: ({ column }) => <SortableCell cellName="Url" column={column} />,
    cell: ({ row }) => <div>{row.original.url}</div>,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.url < rowB.original.url ? 1 : -1;
    },
  },
  {
    id: 'actions',
    cell: ({ row }) => {
      return (
        <div className="text-center">
          <DropdownTableMenu registry={row.original} />
        </div>
      );
    },
  },
];

export const RegistriesTable = () => {
  const registries = useContextSelector(RegistriesContext, (v) => v?.registries) ?? [];
  const isLoading = useContextSelector(RegistriesContext, (v) => v?.isLoading) ?? false;
  const setSelectedRows = useContextSelector(RegistriesContext, (v) => v?.setSelectedRows)!;

  return (
    <>
      {registries?.length > 0 && (
        <div className="flex flex-col gap-3">
          <DataTable columns={columns} data={registries} isLoading={isLoading} onSelectionChange={(ids: string[]) => setSelectedRows(registries.filter((c) => ids.includes(c.id!)))!} />
          <div className="text-muted-foreground text-xs font-normal ">
            {registries?.length > 0 && (
              <span>
                Showing {registries.length} of {registries.length} registries
              </span>
            )}
          </div>
        </div>
      )}
    </>
  );
};
