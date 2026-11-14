import { ChevronDown, ExternalLink } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { useRead } from '@/lib/hooks';

export function DatabaseConnectionsSnapshot({ configuredMaximum }: { configuredMaximum?: number }) {
  const query = useRead('getDatabaseConnections', undefined, {
    refetchOnMount: 'always',
    refetchOnWindowFocus: false,
    retry: false,
  });
  const snapshot = query.data?.data;
  return (
    <div className="max-w-xl space-y-3 border-t pt-4 text-sm">
      <div className="flex flex-wrap items-center justify-between gap-x-6 gap-y-1">
        <span className="text-xs text-muted-foreground">This Core · snapshot</span>
        {snapshot && (
          <p className="tabular-nums">{`${snapshot.open} open · ${snapshot.borrowed} borrowed · ${snapshot.idle} idle`}</p>
        )}
      </div>
      {!snapshot && (
        <div role="status" className="text-muted-foreground">
          {query.isError ? (
            <div className="flex flex-wrap items-center gap-2">
              <p>Could not load pool usage.</p>
              <Button type="button" variant="outline" size="sm" onClick={() => void query.refetch()}>
                Retry database information
              </Button>
            </div>
          ) : (
            'Loading pool usage…'
          )}
        </div>
      )}
      {snapshot && snapshot.maximum !== configuredMaximum && (
        <p className="text-xs text-muted-foreground">Running limit: {snapshot.maximum}</p>
      )}
      <Collapsible>
        <CollapsibleTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="group -ml-2 h-auto max-w-full whitespace-normal text-left text-muted-foreground">
            <ChevronDown
              aria-hidden="true"
              className="size-3.5 transition-transform group-data-[state=open]:rotate-180"
            />
            Advanced database information
          </Button>
        </CollapsibleTrigger>
        <CollapsibleContent className="space-y-3 pt-3 text-sm text-muted-foreground">
          <dl className="flex flex-wrap items-baseline justify-between gap-x-6 gap-y-1">
            <dt>PostgreSQL shared buffers</dt>
            <dd className="font-medium tabular-nums text-foreground">
              {snapshot?.sharedBuffersBytes == null
                ? 'Unavailable'
                : `${(snapshot.sharedBuffersBytes / 1048576).toLocaleString(undefined, { maximumFractionDigits: 2 })} MiB`}
            </dd>
          </dl>
          <p>
            Configured on the PostgreSQL server; not total memory usage. Your database administrator or provider manages
            this setting. Changes require a PostgreSQL restart.
          </p>
          <p>
            The saved connection limit is a shared default. Environment overrides apply to each Core separately. Pool
            counts are approximate and exclude dedicated background connections; leave capacity for those and other
            database clients.
          </p>
          <a
            href="https://docs.citadelplane.com/operations/instance-settings/#database-connections"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1 text-foreground underline underline-offset-4 hover:text-primary">
            Database configuration guide <ExternalLink aria-hidden="true" className="size-3" />
          </a>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
}
