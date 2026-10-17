import { lazy, Suspense, useRef, useState } from 'react';
import { Network } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { CitadelIcons } from '@/lib/icons';
import { ConnectionsSheet } from './connections-sheet';
import type { NetworkOwner } from './map/model';

const WorkloadConnections = lazy(() => import('./map/workload-connections'));

type Workload = { id: string; name: string; platformId?: string | null };

export function WorkloadNetworkAction({ resource, kind }: { resource: Workload; kind: NetworkOwner['kind'] }) {
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const Icon = CitadelIcons[kind];
  if (!resource.platformId) return null;
  return (
    <>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            ref={trigger}
            type="button"
            variant="ghost"
            size="icon"
            className="size-8 shrink-0 text-muted-foreground"
            aria-label="View connections"
            aria-haspopup="dialog"
            aria-expanded={open}
            onClick={() => setOpen(true)}>
            <Network className="size-4" aria-hidden="true" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>View connections</TooltipContent>
      </Tooltip>
      {open && (
        <ConnectionsSheet
          key={`${kind}:${resource.id}:${resource.platformId}`}
          title={resource.name}
          description="Network connections"
          icon={<Icon className="size-5 text-muted-foreground" />}
          onClose={() => setOpen(false)}
          onCloseAutoFocus={(event) => {
            event.preventDefault();
            trigger.current?.focus();
          }}>
          {(maximized) => (
            <Suspense
              fallback={
                <p role="status" className="p-5 text-sm text-muted-foreground">
                  Loading connections…
                </p>
              }>
              <WorkloadConnections
                owner={{ id: resource.id, kind }}
                platformId={resource.platformId!}
                maximized={maximized}
              />
            </Suspense>
          )}
        </ConnectionsSheet>
      )}
    </>
  );
}
