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

export interface DataTableEmptyState {
  title: string;
  description?: string;
}

interface DataTableProps<TData, TValue> {
  columns: ColumnDef<TData, TValue>[];
  data: TData[];
  isLoading: boolean;
  getRowId?: (row: TData) => string;
  getSubRows?: (row: TData) => TData[] | undefined;
  onSelectionChange?: (selectedRows: TData[]) => void;
  emptyState?: DataTableEmptyState;
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
  emptyState,
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
      const selectedModelRows = tableRef.current.getSelectedRowModel().flatRows;
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
                  <div className="flex flex-col items-center justify-center gap-1 p-4 text-center">
                    <div className="text-sm font-medium text-foreground">{emptyState?.title ?? 'No results.'}</div>
                    {emptyState?.description && (
                      <div className="text-xs text-muted-foreground">{emptyState.description}</div>
                    )}
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
