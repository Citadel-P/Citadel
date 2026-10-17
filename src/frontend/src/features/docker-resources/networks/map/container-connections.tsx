import type { ContainerNetworkingView, NetworkContainer } from '@/api/generated/api.types';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { canShowCachedResource } from '@/lib/request-error';
import { useNetworkWorkloads } from '../hooks/useNetworkWorkloads';
import { useNetworksGroup } from '../hooks/useNetworksGroup';
import { WorkloadDetails } from '../workload-details';
import { FocusedMap } from './network-map';

type Props = {
  container: Pick<NetworkContainer, 'containerId' | 'containerName' | 'dockerContainerId' | 'dockerNodeId'>;
  platformId: string;
  maximized: boolean;
};

export default function ContainerConnections(props: Props) {
  const { networking } = useNetworkWorkloads(props.platformId, props.container.containerId, false);
  if (networking.error && !canShowCachedResource(networking.error)) {
    return (
      <div className="p-5">
        <ResourceReadError {...networking} />
      </div>
    );
  }
  const details = networking.data?.data;
  return (
    <>
      {!!networking.error && (
        <div className="p-5">
          <ResourceReadError {...networking} stale />
        </div>
      )}
      {!details ? (
        <p role="status" className="p-5 text-sm text-muted-foreground">
          {networking.isLoading ? 'Loading connections…' : 'Container networking is unavailable.'}
        </p>
      ) : details.mode !== 'networks' ? (
        <div className="min-h-0 flex-1 overflow-y-auto p-5">
          <WorkloadDetails
            container={details.container}
            networking={details}
            platformId={props.platformId}
            visibleNetworks={new Set()}
          />
        </div>
      ) : (
        <ContainerMap {...props} container={details.container} networking={details} stale={!!networking.error} />
      )}
    </>
  );
}

function ContainerMap({
  networking,
  stale,
  ...props
}: Props & { container: NetworkContainer; networking: ContainerNetworkingView; stale: boolean }) {
  const inventory = useNetworksGroup(props.platformId);
  if (inventory.error && !canShowCachedResource(inventory.error)) {
    return (
      <div className="p-5">
        <ResourceReadError {...inventory} />
      </div>
    );
  }
  return (
    <>
      {!!inventory.error && (
        <div className="p-5">
          <ResourceReadError {...inventory} stale />
        </div>
      )}
      <FocusedMap
        showNetworkDetails
        platformId={props.platformId}
        maximized={props.maximized}
        loading={inventory.isLoading}
        workload={{ container: { ...props.container, isStale: props.container.isStale || stale }, networking }}
        networks={(inventory.networks?.networks ?? []).map((network) =>
          stale || inventory.error ? { ...network, isStale: true } : network,
        )}
      />
    </>
  );
}
