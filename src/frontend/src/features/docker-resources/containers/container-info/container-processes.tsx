import { useState, type ReactNode } from 'react';
import { ChevronDown, ListTree } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { useContainerProcesses } from '../hooks/useContainerProcesses';

export function ContainerProcessList({ resourceId }: { resourceId: string }) {
  const { data, isError, isLoading, isFetching, refresh } = useContainerProcesses(resourceId);
  return (
    <div className="border-t">
      {isLoading ? (
        <p role="status" className="px-4 py-8 text-center text-sm text-muted-foreground">
          Loading processes…
        </p>
      ) : isError ? (
        <div className="flex flex-col items-center gap-3 px-4 py-8">
          <p role="alert" className="text-center text-sm text-muted-foreground">
            Processes unavailable.
          </p>
          <Button variant="outline" size="sm" disabled={isFetching} onClick={() => void refresh()}>
            {isFetching ? 'Retrying…' : 'Retry'}
          </Button>
        </div>
      ) : !data?.running ? (
        <p role="status" className="px-4 py-8 text-center text-sm text-muted-foreground">
          Container is stopped.
        </p>
      ) : !data.processes.length ? (
        <p role="status" className="px-4 py-8 text-center text-sm text-muted-foreground">
          No running processes.
        </p>
      ) : (
        <Table aria-label="Container processes">
          <TableHeader>
            <TableRow>
              {data.titles.map((title, index) => (
                <TableHead key={index}>{title === 'CMD' || title === 'COMMAND' ? 'Command' : title}</TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {data.processes.map((row, index) => (
              <TableRow key={index}>
                {data.titles.map((_, column) => (
                  <TableCell key={column} className="font-mono text-xs whitespace-nowrap">
                    {row[column] ?? '—'}
                  </TableCell>
                ))}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
      <p className="border-t bg-muted/10 px-4 py-2.5 text-xs text-muted-foreground">
        Executable names only; command arguments are omitted.
        {data?.truncated && ' Showing the first 1,024 processes.'}
      </p>
    </div>
  );
}

export function ProcessesSection({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <Collapsible open={open} onOpenChange={setOpen} className="rounded-md border bg-background">
      <CollapsibleTrigger className="flex w-full items-center gap-2 rounded-md px-4 py-3 text-sm font-medium hover:bg-muted/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
        <ListTree className="size-4 text-muted-foreground" aria-hidden="true" />
        Processes
        <ChevronDown
          className={`ml-auto size-4 transition-transform motion-reduce:transition-none ${open ? 'rotate-180' : ''}`}
          aria-hidden="true"
        />
      </CollapsibleTrigger>
      <CollapsibleContent>{open && children}</CollapsibleContent>
    </Collapsible>
  );
}

export function ContainerProcesses({ resourceId }: { resourceId: string }) {
  return (
    <ProcessesSection>
      <ContainerProcessList key={resourceId} resourceId={resourceId} />
    </ProcessesSection>
  );
}
