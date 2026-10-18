import { lazy, Suspense, useRef, useState } from 'react';
import { Network } from 'lucide-react';
import type { NetworkContainer } from '@/api/generated/api.types';
import { CitadelIcons } from '@/lib/icons';
import { ConnectionsSheet } from './connections-sheet';

const ContainerConnections = lazy(() => import('./map/container-connections'));

export function ContainerNetworkAction({
  container,
  platformId,
  networks,
}: {
  container: Pick<NetworkContainer, 'containerId' | 'containerName' | 'dockerContainerId' | 'dockerNodeId'>;
  platformId: string;
  networks: Record<string, string>;
}) {
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const names = Object.keys(networks);
  const label =
    names.length === 1
      ? names[0] === 'host'
        ? 'Host network'
        : names[0] === 'none'
          ? 'No network'
          : names[0]
      : names.length
        ? `${names.length} networks`
        : 'View connections';
  return (
    <>
      <button
        ref={trigger}
        type="button"
        className="table-link inline-flex max-w-full items-center gap-1.5 text-sm"
        title={names.join(', ') || undefined}
        aria-label={`View connections: ${label}`}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => setOpen(true)}>
        <Network className="size-3.5 shrink-0" aria-hidden="true" />
        <span className="truncate">{label}</span>
      </button>
      {open && (
        <ConnectionsSheet
          key={`${platformId}:${container.containerId}`}
          title={container.containerName}
          description="Network connections"
          icon={<CitadelIcons.Container className="size-5 text-muted-foreground" />}
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
              <ContainerConnections container={container} platformId={platformId} maximized={maximized} />
            </Suspense>
          )}
        </ConnectionsSheet>
      )}
    </>
  );
}
