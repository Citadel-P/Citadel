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
  const lastSelectionRef = useRef<{ ids?: string; rows?: TData[] }>({});
  const onSelectionChangeRef = useRef(onSelectionChange);

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
    onSelectionChangeRef.current = onSelectionChange;
  }, [onSelectionChange]);

  useEffect(() => {
    const handleSelectionChange = onSelectionChangeRef.current;
    if (handleSelectionChange) {
      const selectedModelRows = tableRef.current.getSelectedRowModel().rows;
      const selectedRowIds = selectedModelRows.map((r) => r.id).join('\u001f');
      const selectedRows = selectedModelRows.map((r) => r.original);
      const lastSelection = lastSelectionRef.current;
      const rowsUnchanged =
        lastSelection.rows?.length === selectedRows.length &&
        lastSelection.rows.every((row, index) => row === selectedRows[index]);
      if (selectedRowIds === lastSelection.ids && rowsUnchanged) return;

      lastSelectionRef.current = { ids: selectedRowIds, rows: selectedRows };
      handleSelectionChange(selectedRows);
    }
  }, [rowSelection, data]);

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
