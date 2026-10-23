import type { StockFeatures, ColumnDef } from '@tanstack/react-table';
import type { ComponentProps, ReactNode } from 'react';
import { TargetCell } from '@/components/custom/common';
import SortableCell from '@/components/custom/sortable-cell';

export type ResourceTargetValue = { target: ComponentProps<typeof TargetCell> } | { label: string; content: ReactNode };

export const createResourceTargetColumn = <T extends object>({
  id,
  header,
  getValue,
}: {
  id: string;
  header: string;
  getValue: (resource: T) => ResourceTargetValue;
}): ColumnDef<StockFeatures, T> => {
  const label = (resource: T) => {
    const value = getValue(resource);
    return 'target' in value ? (value.target.resourceName ?? '') : value.label;
  };
  return {
    id,
    accessorFn: label,
    header: ({ column }) => <SortableCell cellName={header} column={column} />,
    cell: ({ row }) => {
      const value = getValue(row.original);
      return 'target' in value ? <TargetCell {...value.target} /> : value.content;
    },
    sortFn: (rowA, rowB) => label(rowA.original).localeCompare(label(rowB.original)),
  };
};
