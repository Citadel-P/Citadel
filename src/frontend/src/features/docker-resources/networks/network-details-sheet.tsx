import { lazy, Suspense } from 'react';
import type { NetworkView } from '@/api/generated/api.types';
import { ConnectionsSheet } from './connections-sheet';
import { NetworkUsageIcon } from './network-usage-icon';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { canShowCachedResource } from '@/lib/request-error';
import { hasCapability } from '@/lib/resource-capabilities';
import { NetworkDetailsBody } from './network-details';
import { useNetworkInspection } from './hooks/useNetworkInspection';
import { NetworkDetailsFooter } from './network-details-footer';

const NetworkMap = lazy(() => import('./map/network-map'));

// One sheet, one selected-network inspection. Maximizing changes the layout,
// not the mounted graph or its subscriptions, and works without browser fullscreen.
export function NetworkDetailsSheet({
  network: summary,
  networks,
  platformId,
  onClose,
  onCloseAutoFocus,
  context = 'list',
}: {
  network: NetworkView;
  networks: NetworkView[];
  platformId: string;
  onClose: () => void;
  onCloseAutoFocus?: (event: Event) => void;
  context?: 'list' | 'details';
}) {
  const inspection = useNetworkInspection(summary, platformId);
  const inspected = inspection.data?.data ?? summary;
  const network = {
    ...inspected,
    isStale: inspected.isStale || summary.isStale,
    staleReason: inspected.staleReason || summary.staleReason,
  };
  const denied = inspection.error && !canShowCachedResource(inspection.error);
  return (
    <ConnectionsSheet
      title={summary.name}
      description={[summary.driver, summary.scope, summary.nodeHostname].filter(Boolean).join(' · ')}
      icon={<NetworkUsageIcon inUse={summary.inUse} isStale={summary.isStale} className="size-5" />}
      onClose={onClose}
      onCloseAutoFocus={onCloseAutoFocus}>
      {(maximized) =>
        denied ? (
          <div className="p-5">
            <ResourceReadError {...inspection} />
          </div>
        ) : (
          <>
            {!!inspection.error && (
              <div className="p-5">
                <ResourceReadError {...inspection} stale />
              </div>
            )}
            {hasCapability(summary, 'canInspect') ? (
              <Suspense
                fallback={
                  <p role="status" className="p-5 text-sm text-muted-foreground">
                    Loading connections…
                  </p>
                }>
                <NetworkMap
                  network={network}
                  networks={networks}
                  platformId={platformId}
                  loading={inspection.isLoading}
                  maximized={maximized}
                  context={context}
                />
              </Suspense>
            ) : (
              <div className="min-h-0 flex-1 overflow-y-auto p-5">
                <NetworkDetailsBody network={network} platformId={platformId} />
              </div>
            )}
            {context === 'list' && <NetworkDetailsFooter network={network} platformId={platformId} />}
          </>
        )
      }
    </ConnectionsSheet>
  );
}
