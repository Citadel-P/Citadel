import { Skeleton } from './skeleton';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from './table';

export function TableSkeletonRows({ columns = 5 }: { columns?: number }) {
  return Array.from({ length: 5 }, (_, row) => (
    <TableRow key={row} aria-hidden="true">
      {Array.from({ length: columns }, (_, column) => (
        <TableCell key={column}>
          <Skeleton className={`h-3 max-w-40 ${(row + column) % 3 === 0 ? 'w-1/2' : 'w-3/4'}`} />
        </TableCell>
      ))}
    </TableRow>
  ));
}

export function TableSkeleton() {
  return (
    <Table aria-hidden="true">
      <TableHeader>
        <TableRow>
          {Array.from({ length: 5 }, (_, column) => (
            <TableHead key={column}>
              <Skeleton className="h-3 w-16" />
            </TableHead>
          ))}
        </TableRow>
      </TableHeader>
      <TableBody>
        <TableSkeletonRows />
      </TableBody>
    </Table>
  );
}
