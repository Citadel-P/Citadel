import { StockFeatures, Column } from '@tanstack/react-table';
import { ArrowUpDown } from 'lucide-react';

interface SortButtonProps<T extends object> {
  cellName: string;
  column: Column<StockFeatures, T, unknown>;
}

const SortableCell = <T extends object>({ column, cellName }: SortButtonProps<T>) => {
  return (
    <div className="flex justify-between group">
      <span>{cellName}</span>
      <button
        className="invisible group-hover:visible"
        onClick={() => column.toggleSorting(column.getIsSorted() === 'asc')}>
        <ArrowUpDown className="ml-2 h-4 w-4" />
      </button>
    </div>
  );
};

export default SortableCell;
