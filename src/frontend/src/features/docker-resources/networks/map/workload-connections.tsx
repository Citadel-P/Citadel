import { ResourceReadError } from '@/components/custom/resource-read-error';
import { canShowCachedResource } from '@/lib/request-error';
import { useNetworksGroup } from '../hooks/useNetworksGroup';
import { FocusedMap } from './network-map';
import type { NetworkOwner } from './model';

export default function WorkloadConnections({
  owner,
  platformId,
  maximized,
}: {
  owner: NetworkOwner;
  platformId: string;
  maximized: boolean;
}) {
  const query = useNetworksGroup(platformId, true);
  if (query.error && !canShowCachedResource(query.error)) {
    return (
      <div className="p-5">
        <ResourceReadError {...query} />
      </div>
    );
  }
  return (
    <>
      {!!query.error && (
        <div className="p-5">
          <ResourceReadError {...query} stale />
        </div>
      )}
      <FocusedMap
        showNetworkDetails
        owner={owner}
        platformId={platformId}
        maximized={maximized}
        loading={query.isLoading}
        networks={(query.networks?.networks ?? []).map((network) =>
          query.error ? { ...network, isStale: true } : network,
        )}
      />
    </>
  );
}
