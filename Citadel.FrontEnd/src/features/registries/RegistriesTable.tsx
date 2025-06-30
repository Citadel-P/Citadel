import { DataTable } from '@/components/ui/data-table';
import { RegistryView } from '@/api/_generated';
import SortableCell from '@/components/ui/SortableCell';
import DropdownTableMenu from './DropdownTableMenu';
import { ColumnDef, Row } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useRegistriesContext } from './RegistriesProvider';
import { Link } from 'react-router';
import { InfoIcon } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

const getNonDefaultRows = (rows: Row<RegistryView>[]) => rows.filter((row) => !row.original.isDefault);

const areAllNonDefaultRowsSelected = (rows: Row<RegistryView>[]) =>
  getNonDefaultRows(rows).every((row) => row.getIsSelected());

const areSomeNonDefaultRowsSelected = (rows: Row<RegistryView>[]) =>
  getNonDefaultRows(rows).some((row) => row.getIsSelected());

const columns: ColumnDef<RegistryView>[] = [
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
          aria-label="Select registry"
        />
      );
    },

    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => {
      if (row.original.isDefault) {
        return (
          <div className="flex items-center gap-1">
            <span>{row.original.name}</span>
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
          </div>
        );
      }

      return (
        <Link to={`../registries/edit/${row.original.id}`} className="hover:underline">
          {row.original.name}
        </Link>
      );
    },
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => rowA.original.name.localeCompare(rowB.original.name),
  },
  {
    accessorKey: 'provider',
    header: ({ column }) => <SortableCell cellName="Provider" column={column} />,
    cell: ({ row }) => <div>{row.original.type}</div>,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.type.localeCompare(rowB.original.type);
    },
  },
  {
    accessorKey: 'url',
    header: ({ column }) => <SortableCell cellName="Url" column={column} />,
    cell: ({ row }) => <div>{row.original.url}</div>,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.url.localeCompare(rowB.original.url);
    },
  },
  {
    id: 'actions',
    cell: ({ row }) => {
      if (row.original.isDefault) {
        return <></>;
      }
      return (
        <div className="text-center">
          <DropdownTableMenu registry={row.original} />
        </div>
      );
    },
  },
];

export const RegistriesTable = () => {
  const { registries, isLoading, setSelectedRows } = useRegistriesContext();

  if (!registries?.length) {
    return null;
  }

  return (
    <div className="flex flex-col gap-3">
      <DataTable
        columns={columns}
        data={registries}
        isLoading={isLoading}
        onSelectionChange={(ids: string[]) => setSelectedRows(registries.filter((c) => ids.includes(c.id!)))!}
      />
      <div className="text-muted-foreground text-xs font-normal ">
        <span>
          Showing {registries.length} of {registries.length} registries
        </span>
      </div>
    </div>
  );
};
