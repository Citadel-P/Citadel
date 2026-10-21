import type { ContainerNetworkingView, NetworkContainer } from '@/api/generated/api.types';
import { Link } from 'react-router';
import { WorkloadLink } from './workload-link';

export function networkModeLabel(mode: string) {
  switch (mode) {
    case 'host':
      return 'Uses host network';
    case 'none':
      return 'No network';
    case 'shared':
      return 'Shares another container’s network namespace. Target details are unavailable.';
    case 'networks':
      return 'Networks';
    default:
      return 'Network mode is unavailable';
  }
}
export function WorkloadDetails({
  container,
  networking,
  platformId,
  visibleNetworks,
}: {
  container: NetworkContainer;
  networking: ContainerNetworkingView;
  platformId: string;
  visibleNetworks: Set<string>;
}) {
  return (
    <div className="space-y-5">
      <Link className="table-link block text-sm" to={`/platforms/${platformId}/containers/${container.containerId}`}>
        Open container
      </Link>
      <WorkloadLink entry={container} />
      <p className="text-sm">{networkModeLabel(networking.mode)}</p>
      {networking.mode === 'networks' && (
        <ul className="divide-y">
          {networking.endpoints.map((endpoint, index) => (
            <li key={`${endpoint.networkId}:${index}`} className="space-y-1 py-3">
              <p className="break-all text-sm font-medium">{endpoint.name}</p>
              {endpoint.attached !== true && (
                <p className="text-xs text-muted-foreground">
                  {endpoint.attached === false
                    ? 'Configured network · Not attached'
                    : 'Attachment evidence unavailable'}
                </p>
              )}
              {endpoint.attached === true && !visibleNetworks.has(endpoint.networkId) && (
                <p className="text-xs text-muted-foreground">Outside these results or unavailable in inventory</p>
              )}
              <p className="break-all font-mono text-xs text-muted-foreground">
                {endpoint.attached === true
                  ? endpoint.addresses.join(' · ') || 'Address unavailable'
                  : endpoint.attached === false
                    ? 'No active address'
                    : 'Address unavailable'}
              </p>
              {!!endpoint.aliases.length && (
                <p className="break-all text-xs text-muted-foreground">Aliases: {endpoint.aliases.join(', ')}</p>
              )}
            </li>
          ))}
        </ul>
      )}
      {!!networking.publishedPorts.length && (
        <section className="space-y-2">
          <h3 className="text-sm font-medium">Published ports</h3>
          {networking.publishedPorts.map((port, i) => (
            <p key={i} className="break-all font-mono text-xs text-muted-foreground">
              {port.hostIp || '*'}:{port.hostPort} → {port.containerPort}
            </p>
          ))}
        </section>
      )}
      <p className="text-xs text-muted-foreground">
        Observed {new Date(networking.observedAt * 1000).toLocaleString()}
      </p>
      {networking.truncated && <p className="text-xs text-muted-foreground">Some networking details are omitted.</p>}
    </div>
  );
}
