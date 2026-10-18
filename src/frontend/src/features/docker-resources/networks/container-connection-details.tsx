import type { NetworkUsageEntry } from '@/api/generated/api.types';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { canShowCachedResource } from '@/lib/request-error';
import { useNetworkWorkloads } from './hooks/useNetworkWorkloads';
import { networkModeLabel } from './workload-details';

// Mounted for one selected endpoint only, in either List or Map.
export function ContainerConnectionDetails({ entry, platformId }: { entry: NetworkUsageEntry; platformId: string }) {
  const { networking } = useNetworkWorkloads(platformId, entry.containerId ?? undefined, false);
  if (networking.error && !canShowCachedResource(networking.error)) return <ResourceReadError {...networking} />;
  const details = networking.data?.data;
  return (
    <div className="space-y-2 rounded-md border p-3 text-xs text-muted-foreground">
      {!!networking.error && <ResourceReadError {...networking} stale />}
      {networking.isLoading && <p role="status">Loading connection details…</p>}
      {details && (
        <>
          <p>{networkModeLabel(details.mode)}</p>
          {details.mode === 'networks' &&
            details.endpoints.map((endpoint, index) => (
              <div key={index} className="space-y-1">
                <p className="font-medium text-foreground">{endpoint.name}</p>
                {endpoint.attached !== true && (
                  <p className="text-xs text-muted-foreground">
                    {endpoint.attached === false
                      ? 'Configured network · Not attached'
                      : 'Attachment evidence unavailable'}
                  </p>
                )}
                <p className="break-all font-mono">{endpoint.addresses.join(' · ') || 'Address unavailable'}</p>
                {!!endpoint.aliases.length && <p className="break-all">Aliases: {endpoint.aliases.join(', ')}</p>}
              </div>
            ))}
          {!!details.publishedPorts.length && (
            <div>
              <p className="font-medium text-foreground">Published ports</p>
              {details.publishedPorts.map((port, index) => (
                <p key={index} className="break-all font-mono">
                  {port.hostIp || '*'}:{port.hostPort} → {port.containerPort}
                </p>
              ))}
            </div>
          )}
          <p>Observed {new Date(details.observedAt * 1000).toLocaleString()}</p>
          {details.truncated && <p>Some networking details are omitted.</p>}
        </>
      )}
    </div>
  );
}
