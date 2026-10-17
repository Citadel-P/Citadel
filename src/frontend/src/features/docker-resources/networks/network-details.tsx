import { useState } from 'react';
import { Link } from 'react-router';
import { ChevronDown, Network } from 'lucide-react';
import type { NetworkView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { StateIndicator } from '@/components/custom/state-indicator';
import { WorkloadLink } from './workload-link';
import { ContainerConnectionDetails } from './container-connection-details';
import { networkAddresses, networkProperties, networkUsageLabel } from './network-presentation';

// Pure presentation shared by the sheet and graph selections; the caller owns inspection.
export function NetworkDetailsBody({
  network,
  platformId,
  onExplore,
}: {
  network: NetworkView;
  platformId?: string;
  onExplore?: (id: string) => void;
}) {
  const [detailsId, setDetailsId] = useState<string>();
  const attachments = network.usage?.entries.filter((entry) => entry.relationship === 'Attachment') ?? [];
  const references = network.usage?.entries.filter((entry) => entry.relationship === 'SavedReference') ?? [];
  const properties = networkProperties(network);
  const facts = [
    { label: 'Subnets', value: networkAddresses(network, 'subnet').join(', ') || '—' },
    { label: 'Gateway', value: networkAddresses(network, 'gateway').join(', ') || '—' },
    ...(properties.length ? [{ label: 'Properties', value: properties.join(', ') }] : []),
    ...(network.dockerNodeId ? [{ label: 'Node', value: network.nodeHostname || network.dockerNodeId }] : []),
  ];
  return (
    <div className="space-y-6">
      {network.isStale && (
        <p role="status" className="text-sm text-muted-foreground">
          {network.staleReason || 'Network information is out of date.'}
        </p>
      )}
      <dl className="space-y-3 border-b pb-5">
        {facts.map(({ label, value }) => (
          <div key={label} className="grid grid-cols-[5rem_minmax(0,1fr)] items-baseline gap-4">
            <dt className="text-xs text-muted-foreground">{label}</dt>
            <dd className="min-w-0 text-sm [overflow-wrap:anywhere]">{value}</dd>
          </div>
        ))}
      </dl>
      <section className="space-y-3">
        <div className="flex items-center gap-2">
          <h2 className="text-sm font-semibold">Connected to network</h2>
          <Badge
            variant="secondary"
            className="h-5 min-w-5 border-transparent bg-muted px-1.5 py-0 font-normal tabular-nums text-muted-foreground"
            aria-label="Visible connected containers">
            {attachments.length}
          </Badge>
        </div>
        {attachments.length ? (
          <ul aria-label="Connected containers" className="divide-y rounded-md border px-3">
            {attachments.map((entry) => {
              const attachment = network.containers?.[entry.dockerContainerId ?? ''];
              return (
                <li key={entry.containerId} className="flex items-start gap-3 py-3">
                  <StateIndicator
                    kind="container"
                    value={entry.isStale || network.isStale ? 'Unknown' : (entry.containerState ?? 'Unknown')}
                    tooltip={entry.isStale || network.isStale ? 'Container state is out of date' : undefined}
                    className="mt-1.5 mr-0"
                  />
                  <div className="min-w-0 flex-1 space-y-2">
                    {platformId && entry.containerId ? (
                      <Link
                        className="table-link block break-all text-sm font-medium"
                        to={`/platforms/${platformId}/containers/${entry.containerId}`}>
                        {entry.containerName}
                      </Link>
                    ) : (
                      <p className="text-sm">{entry.containerName}</p>
                    )}
                    <p className="break-all font-mono text-xs text-muted-foreground">
                      {[attachment?.ipV4Address, attachment?.ipv6Address].filter(Boolean).join(' · ') ||
                        'Address unavailable'}
                    </p>
                    <WorkloadLink entry={entry} />
                    <div className="-ml-2 flex flex-wrap items-center gap-x-1 gap-y-1">
                      {platformId && entry.containerId && (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-7 gap-1.5 px-2 text-xs text-muted-foreground"
                          onClick={() => setDetailsId(detailsId === entry.containerId ? undefined : entry.containerId!)}
                          aria-expanded={detailsId === entry.containerId}>
                          <ChevronDown
                            aria-hidden="true"
                            className={`size-3.5 transition-transform ${detailsId === entry.containerId ? 'rotate-180' : ''}`}
                          />
                          Connection details
                        </Button>
                      )}
                      {onExplore && entry.containerId && (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-7 gap-1.5 px-2 text-xs text-muted-foreground"
                          onClick={() => onExplore(entry.containerId!)}>
                          <Network className="size-3.5" aria-hidden="true" />
                          Explore connections
                        </Button>
                      )}
                    </div>
                    {platformId && detailsId === entry.containerId && (
                      <ContainerConnectionDetails entry={entry} platformId={platformId} />
                    )}
                  </div>
                </li>
              );
            })}
          </ul>
        ) : (
          <p className="text-sm text-muted-foreground">{networkUsageLabel(network)}</p>
        )}
      </section>
      {!!network.usage?.entries.some((entry) => entry.relationship === 'ServiceNetwork') && (
        <section className="space-y-3">
          <h2 className="text-sm font-semibold">Swarm service networks</h2>
          {network.usage.entries
            .filter((entry) => entry.relationship === 'ServiceNetwork')
            .map((entry) => (
              <div key={entry.dockerServiceId} className="space-y-1 text-sm">
                <p>
                  {entry.serviceName}
                  {entry.isStale ? ' · Out of date' : ''}
                </p>
                <WorkloadLink entry={entry} />
                {platformId && entry.dockerServiceId && (
                  <Link
                    className="table-link block text-xs"
                    to={`/platforms/${platformId}/services/${entry.dockerServiceId}`}>
                    Open service
                  </Link>
                )}
                <p className="text-xs text-muted-foreground">
                  Observed service configuration; task coverage may be incomplete.
                </p>
              </div>
            ))}
        </section>
      )}
      {references.length > 0 && (
        <section className="space-y-3">
          <h2 className="text-sm font-semibold">Configured workloads</h2>
          <ul className="divide-y rounded-md border px-3">
            {references.map((entry) => (
              <li key={`${entry.workloadKind}:${entry.workloadId}:${entry.reference}`} className="space-y-1 py-3">
                <WorkloadLink entry={entry} />
                <p className="text-xs text-muted-foreground">
                  {entry.workloadKind} · {entry.reference}
                </p>
              </li>
            ))}
          </ul>
          <p className="text-xs text-muted-foreground">
            These workloads are configured to use this network, but may not be attached.
          </p>
        </section>
      )}
      <p className="text-xs text-muted-foreground">
        {network.usage?.truncated ? 'Some results are omitted. ' : ''}
        {network.scope === 'swarm' || network.dockerNodeId
          ? 'Attachments reflect this node’s observation, not the whole Swarm. '
          : ''}
        Coverage may be incomplete. Only accessible workloads and explicit saved network names are shown.
      </p>
    </div>
  );
}
