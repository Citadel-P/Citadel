// This component has been patched:
// - Add className='group/rowid' to customize the copyToCliboard functionality
// - Expose getRowId & onSelectionChange
import {
  ColumnDef,
  ExpandedState,
  RowSelectionState,
  SortingState,
  flexRender,
  getCoreRowModel,
  getExpandedRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';

import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { useEffect, useRef, useState } from 'react';
import Loader from './loader';

interface DataTableProps<TData, TValue> {
  columns: ColumnDef<TData, TValue>[];
  data: TData[];
  isLoading: boolean;
  getRowId?: (row: TData) => string;
  getSubRows?: (row: TData) => TData[] | undefined;
  onSelectionChange?: (selectedRows: TData[]) => void;
}
interface Identifiable {
  id?: string | null;
}
export function DataTable<TData extends Identifiable, TValue>({
  columns,
  data,
  isLoading,
  getRowId,
  getSubRows,
  onSelectionChange,
}: DataTableProps<TData, TValue>) {
  const [sorting, setSorting] = useState<SortingState>([]);
  const [rowSelection, setRowSelection] = useState<RowSelectionState>({});
  const [expanded, setExpanded] = useState<ExpandedState>({});

  const table = useReactTable({
    data,
    columns,
    getRowId: (row) => (getRowId ? getRowId(row) : row.id!),
    getSubRows,
    getCoreRowModel: getCoreRowModel(),
    onRowSelectionChange: setRowSelection,
    onSortingChange: setSorting,
    onExpandedChange: setExpanded,
    getSortedRowModel: getSortedRowModel(),
    getExpandedRowModel: getSubRows ? getExpandedRowModel() : undefined,
    state: {
      sorting,
      rowSelection,
      expanded,
    },
  });

  const tableRef = useRef(table);
  tableRef.current = table;

  useEffect(() => {
    if (onSelectionChange) {
      const selectedRows = tableRef.current.getSelectedRowModel().rows.map((r) => r.original);
      onSelectionChange(selectedRows);
    }
  }, [rowSelection, data, onSelectionChange]);

  return (
    <div className="rounded-none">
      <Table>
        <TableHeader>
          {table.getHeaderGroups().map((headerGroup) => (
            <TableRow key={headerGroup.id}>
              {headerGroup.headers.map((header) => {
                return (
                  <TableHead key={header.id}>
                    {header.isPlaceholder ? null : flexRender(header.column.columnDef.header, header.getContext())}
                  </TableHead>
                );
              })}
            </TableRow>
          ))}
        </TableHeader>
        <TableBody>
          {table.getRowModel().rows?.length ? (
            table.getRowModel().rows.map((row) => (
              <TableRow key={row.id} data-state={row.getIsSelected() && 'selected'} className="group/rowid">
                {row.getVisibleCells().map((cell) => (
                  <TableCell key={cell.id}>{flexRender(cell.column.columnDef.cell, cell.getContext())}</TableCell>
                ))}
              </TableRow>
            ))
          ) : (
            <TableRow className="group">
              <TableCell colSpan={columns.length} className="h-24">
                {isLoading && <Loader />}
                {!isLoading && (
                  <div className="flex items-center justify-center p-4">
                    <div className="rounded-full bg-muted/20 text-forground/90 px-3 py-1 text-center">No results.</div>
                  </div>
                )}
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
    </div>
  );
}
