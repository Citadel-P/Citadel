import { Column } from '@tanstack/react-table';
import { ArrowUpDown } from 'lucide-react';

interface SortButtonProps<T> {
  cellName: string;
  column: Column<T, unknown>;
}

const SortableCell = <T,>({ column, cellName }: SortButtonProps<T>) => {
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
