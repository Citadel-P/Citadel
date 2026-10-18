import { useCallback, useMemo, useState } from 'react';
import { Link, useSearchParams } from 'react-router';
import { ArrowLeft, Check, ChevronsUpDown, Ellipsis, RotateCcw, Box } from 'lucide-react';
import type { NetworkView, NetworkContainer, ContainerNetworkingView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogTitle, DialogDescription } from '@/components/ui/dialog';
import { Command, CommandEmpty, CommandInput, CommandItem, CommandList } from '@/components/ui/command';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuCheckboxItem,
  DropdownMenuItem,
} from '@/components/ui/dropdown-menu';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { canShowCachedResource } from '@/lib/request-error';
import { cn } from '@/lib/utils';
import { CitadelIcons } from '@/lib/icons';
import { NetworkDetailsBody } from '../network-details';
import { WorkloadLink } from '../workload-link';
import { networkGraph, workloadGraph, type MapNode, type NetworkOwner, ownerGraph } from './model';
import NetworkCanvas from './canvas';
import { useNetworkWorkloads } from '../hooks/useNetworkWorkloads';
import { WorkloadDetails } from '../workload-details';
import { ContainerConnectionDetails } from '../container-connection-details';
import { MapNodeIcon } from './node-icon';
import { NetworkDetailsFooter } from '../network-details-footer';

type MapProps = {
  network: NetworkView;
  networks: NetworkView[];
  platformId: string;
  loading: boolean;
  maximized: boolean;
  context?: 'list' | 'details';
};

// The sheet already owns the network inspection. Workload reads are mounted
// only after an explicit exploration, never for each visible graph node.
export default function NetworkMap(props: MapProps) {
  const [picking, setPicking] = useState(false);
  const [params, setParams] = useSearchParams();
  if (props.context === 'details') return <FocusedMap {...props} />;
  const containerId = params.get('networkPlatform') === props.platformId ? params.get('networkContainer') : null;
  const explore = (id?: string) =>
    setParams((current) => {
      const next = new URLSearchParams(current);
      if (id) next.set('networkContainer', id);
      else next.delete('networkContainer');
      return next;
    });
  if (containerId) return <WorkloadMap {...props} containerId={containerId} onBack={() => explore()} />;
  return (
    <>
      <FocusedMap {...props} onExplore={explore} onFindWorkload={() => setPicking(true)} />
      {picking && (
        <WorkloadPicker
          platformId={props.platformId}
          onClose={() => setPicking(false)}
          onSelect={(id) => {
            setPicking(false);
            explore(id);
          }}
        />
      )}
    </>
  );
}

function WorkloadPicker({
  platformId,
  onSelect,
  onClose,
}: {
  platformId: string;
  onSelect: (id: string) => void;
  onClose: () => void;
}) {
  const { catalog } = useNetworkWorkloads(platformId);
  const denied = catalog.error && !canShowCachedResource(catalog.error);
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}>
      <DialogContent className="gap-0 overflow-hidden p-0">
        <div className="space-y-1 p-4 pr-10">
          <DialogTitle>Explore workload</DialogTitle>
          <DialogDescription>See a container’s network connections.</DialogDescription>
        </div>
        {!!catalog.error && <ResourceReadError {...catalog} stale={!denied} />}
        {!denied && (
          <Command>
            <CommandInput placeholder="Find a container or owner…" aria-label="Find workload" />
            <CommandList>
              <CommandEmpty>{catalog.isLoading ? 'Loading workloads…' : 'No matching workloads.'}</CommandEmpty>
              {(catalog.data?.data.containers ?? []).map((container) => {
                const Icon = CitadelIcons[container.workloadKind ?? 'Container'];
                return (
                  <CommandItem
                    key={container.containerId}
                    value={container.containerId}
                    keywords={[container.containerName, container.workloadName ?? '', container.dockerContainerId]}
                    onSelect={() => onSelect(container.containerId)}>
                    <Icon className="size-4 shrink-0" />
                    <div className="min-w-0">
                      <p className="truncate">{container.containerName}</p>
                      <p className="truncate text-xs text-muted-foreground">
                        {[container.workloadName, container.dockerNodeId].filter(Boolean).join(' · ')}
                      </p>
                    </div>
                  </CommandItem>
                );
              })}
            </CommandList>
          </Command>
        )}
        {catalog.data?.data.truncated && !denied && (
          <p className="p-3 text-xs text-muted-foreground">Showing the first 500 accessible workloads.</p>
        )}
      </DialogContent>
    </Dialog>
  );
}

function WorkloadMap({ containerId, onBack, ...props }: MapProps & { containerId: string; onBack: () => void }) {
  const { networking } = useNetworkWorkloads(props.platformId, containerId, false);
  const denied = networking.error && !canShowCachedResource(networking.error);
  const container = networking.data?.data.container;
  return (
    <>
      <div className="shrink-0 border-b px-4 py-2">
        <Button variant="ghost" size="sm" className="max-w-full" onClick={onBack}>
          <ArrowLeft className="size-4 shrink-0" />
          <span className="truncate">Back to {props.network.name}</span>
        </Button>
      </div>
      {denied ? (
        <div className="p-5">
          <ResourceReadError {...networking} />
        </div>
      ) : (
        <>
          {!!networking.error && <ResourceReadError {...networking} stale />}
          {container && networking.data?.data ? (
            <FocusedMap
              {...props}
              loading={false}
              workload={{
                container: { ...container, isStale: container.isStale || !!networking.error },
                networking: networking.data.data,
              }}
            />
          ) : (
            <p role="status" className="p-5 text-sm text-muted-foreground">
              {networking.isLoading ? 'Loading connections…' : 'This workload is no longer available.'}
            </p>
          )}
        </>
      )}
    </>
  );
}

export function FocusedMap({
  network,
  platformId,
  loading,
  maximized,
  workload,
  networks,
  onExplore,
  onFindWorkload,
  owner,
  showNetworkDetails = false,
}: Omit<MapProps, 'network'> & {
  network?: NetworkView;
  owner?: NetworkOwner;
  showNetworkDetails?: boolean;
  workload?: { container: NetworkContainer; networking: ContainerNetworkingView };
  onExplore?: (id: string) => void;
  onFindWorkload?: () => void;
}) {
  const [references, setReferences] = useState(false);
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());
  const [selectedId, setSelectedId] = useState<string>();
  const [reset, setReset] = useState(0);
  const graph = useMemo(
    () =>
      owner
        ? ownerGraph(platformId, owner, networks, references, expanded)
        : workload
          ? workloadGraph(platformId, workload.container, workload.networking, networks)
          : network
            ? networkGraph(platformId, network, references, expanded)
            : { nodes: [], edges: [], truncated: false },
    [platformId, network, references, expanded, workload, networks, owner],
  );
  const selected = graph.nodes.find((node) => node.id === selectedId) ?? graph.nodes[0];
  const detailNetwork = selected?.network ?? graph.nodes.find((node) => node.network)?.network;
  const toggle = useCallback(
    (id: string) =>
      setExpanded((current) => {
        const next = new Set(current);
        if (next.has(id)) next.delete(id);
        else next.add(id);
        return next;
      }),
    [],
  );
  return (
    <>
      <div className={cn('min-h-0 flex-1 overflow-y-auto', maximized && 'lg:flex lg:flex-col lg:overflow-hidden')}>
        <div className="flex shrink-0 items-center gap-3 border-b px-5 py-3">
          <h2 className="mr-auto text-sm font-semibold">Connections</h2>
          {selected && <MapSelection nodes={graph.nodes} selected={selected} onSelect={setSelectedId} />}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" className="size-8 shrink-0" aria-label="Map options">
                <Ellipsis className="size-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              {onFindWorkload && (
                <DropdownMenuItem onSelect={onFindWorkload}>
                  <Box />
                  Explore workload…
                </DropdownMenuItem>
              )}
              {!workload && (
                <DropdownMenuCheckboxItem checked={references} onCheckedChange={setReferences}>
                  {owner ? 'Show configured networks' : 'Show configured workloads'}
                </DropdownMenuCheckboxItem>
              )}
              <DropdownMenuItem onSelect={() => setReset((value) => value + 1)}>
                <RotateCcw />
                Reset layout
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
        <div className={cn('min-h-0', maximized && 'lg:grid lg:flex-1 lg:grid-cols-[minmax(0,1fr)_340px]')}>
          <div className={cn('flex min-w-0 flex-col', maximized && 'lg:min-h-0')}>
            <div className={cn('h-[340px] min-h-[300px]', maximized && 'lg:h-auto lg:flex-1')}>
              {loading ? (
                <p role="status" className="p-5 text-sm text-muted-foreground">
                  Loading attachments…
                </p>
              ) : !selected ? (
                <p role="status" className="p-5 text-sm text-muted-foreground">
                  No network connections reported for this workload.
                </p>
              ) : (
                <NetworkCanvas
                  graph={graph}
                  selected={selected.id}
                  onSelect={setSelectedId}
                  onToggle={toggle}
                  reset={reset}
                />
              )}
            </div>
            <div className="flex shrink-0 flex-wrap items-center gap-x-4 gap-y-1 border-y px-5 py-2 text-[11px] text-muted-foreground">
              <span className="inline-flex items-center gap-2">
                <span className="w-4 border-t border-muted-foreground/60" />
                Network connection
              </span>
              {references && (
                <span className="inline-flex items-center gap-2">
                  <span className="w-4 border-t border-dashed border-muted-foreground/60" />
                  Configured use
                </span>
              )}
              <span className="ml-auto">
                {graph.nodes.some((node) => node.isStale) ? 'Out of date' : 'Application connectivity isn’t tested.'}
              </span>
              {(graph.truncated ||
                graph.nodes.some((node) => node.network?.scope === 'swarm' || node.network?.dockerNodeId)) && (
                <span className="w-full">
                  {graph.truncated ? 'Some relationships are omitted. ' : ''}
                  {graph.nodes.some((node) => node.network?.scope === 'swarm' || node.network?.dockerNodeId)
                    ? owner
                      ? 'Swarm task coverage may be incomplete.'
                      : 'This node’s observations only.'
                    : ''}
                </span>
              )}
            </div>
          </div>
          {selected && (
            <aside
              aria-label="Map selection details"
              className={cn('space-y-5 p-5', maximized && 'lg:min-h-0 lg:overflow-y-auto lg:border-l')}>
              <div className="flex items-start gap-3">
                <MapNodeIcon node={selected} className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                <div className="min-w-0">
                  <h3 className="break-all text-sm font-semibold">{selected.label}</h3>
                  <p className="mt-1 text-xs text-muted-foreground">{selected.subtitle}</p>
                </div>
              </div>
              {selected.kind === 'network' && selected.network ? (
                <NetworkDetailsBody network={selected.network} platformId={platformId} onExplore={onExplore} />
              ) : owner ? (
                <OwnerMemberDetails node={selected} platformId={platformId} />
              ) : workload ? (
                <WorkloadDetails
                  {...workload}
                  platformId={platformId}
                  visibleNetworks={new Set(graph.nodes.flatMap((node) => (node.network ? [node.network.id] : [])))}
                />
              ) : network ? (
                <MemberDetails node={selected} network={network} platformId={platformId} onExplore={onExplore} />
              ) : null}
            </aside>
          )}
        </div>
      </div>
      {showNetworkDetails && <NetworkDetailsFooter network={detailNetwork} platformId={platformId} />}
    </>
  );
}

function MapSelection({
  nodes,
  selected,
  onSelect,
}: {
  nodes: MapNode[];
  selected: MapNode;
  onSelect: (id: string) => void;
}) {
  const [open, setOpen] = useState(false);
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          role="combobox"
          aria-label="Map selection"
          aria-expanded={open}
          className="h-8 min-w-0 max-w-[200px] gap-2 text-xs shadow-none">
          <span className="truncate">{selected.label}</span>
          <ChevronsUpDown className="size-3.5 shrink-0 text-muted-foreground" />
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-[min(320px,calc(100vw-32px))] p-0">
        <Command>
          <CommandInput placeholder="Find a resource or alias…" aria-label="Find in map" />
          <CommandList>
            <CommandEmpty>No matching resources.</CommandEmpty>
            {nodes.map((node) => {
              return (
                <CommandItem
                  key={node.id}
                  value={node.id}
                  keywords={[node.label, node.subtitle, node.address ?? '', ...(node.searchTerms ?? [])]}
                  onSelect={() => {
                    onSelect(node.id);
                    setOpen(false);
                  }}>
                  <MapNodeIcon node={node} className="size-4 shrink-0" />
                  <span className="min-w-0 flex-1 truncate">{node.label}</span>
                  {node.id === selected.id && <Check className="size-4" />}
                </CommandItem>
              );
            })}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}

function MemberDetails({
  node,
  network,
  platformId,
  onExplore,
}: {
  node: MapNode;
  network: NetworkView;
  platformId: string;
  onExplore?: (id: string) => void;
}) {
  return (
    <div className="space-y-4">
      {node.kind === 'container' && node.entries[0]?.containerId && (
        <ContainerConnectionDetails entry={node.entries[0]} platformId={platformId} />
      )}
      {node.kind === 'reference' && (
        <p className="text-xs text-muted-foreground">Configured to use this network; connection not confirmed.</p>
      )}
      <ul className="divide-y">
        {node.entries.map((entry, index) => {
          const endpoint = network.containers?.[entry.dockerContainerId ?? ''];
          return (
            <li key={JSON.stringify([entry.containerId, entry.reference, index])} className="space-y-2 py-3 first:pt-0">
              {entry.containerId && (
                <Link
                  className="table-link block break-all text-sm"
                  to={`/platforms/${platformId}/containers/${entry.containerId}`}>
                  {entry.containerName || 'Open container'}
                </Link>
              )}
              <WorkloadLink entry={entry} />
              {entry.dockerServiceId && (
                <Link
                  className="table-link block text-xs"
                  to={`/platforms/${platformId}/services/${entry.dockerServiceId}`}>
                  Open service
                </Link>
              )}
              {entry.containerId && onExplore && (
                <Button variant="outline" size="sm" onClick={() => onExplore(entry.containerId!)}>
                  Explore connections
                </Button>
              )}
              {entry.relationship === 'ServiceNetwork' && (
                <p className="text-xs text-muted-foreground">
                  Service network configuration{entry.isStale ? ' · Out of date' : ''}. Task coverage may be incomplete.
                </p>
              )}
              {entry.observedAt && (
                <p className="text-xs text-muted-foreground">
                  Observed {new Date(entry.observedAt * 1000).toLocaleString()}
                </p>
              )}
              {entry.relationship === 'Attachment' && (
                <p className="break-all font-mono text-xs text-muted-foreground">
                  {[endpoint?.ipV4Address, endpoint?.ipv6Address].filter(Boolean).join(' · ') || 'Address unavailable'}
                </p>
              )}
              {entry.reference && <p className="break-all text-xs text-muted-foreground">{entry.reference}</p>}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

function OwnerMemberDetails({ node, platformId }: { node: MapNode; platformId: string }) {
  if (node.kind === 'container' && node.entries[0]?.containerId) {
    return (
      <ContainerConnectionDetails key={node.entries[0].containerId} entry={node.entries[0]} platformId={platformId} />
    );
  }
  return (
    <ul className="space-y-3 text-sm">
      {node.entries.map((entry, index) => (
        <li key={index} className="space-y-1">
          {entry.containerId ? (
            <Link className="table-link" to={`/platforms/${platformId}/containers/${entry.containerId}`}>
              {entry.containerName || 'Open container'}
            </Link>
          ) : entry.dockerServiceId ? (
            <Link className="table-link" to={`/platforms/${platformId}/services/${entry.dockerServiceId}`}>
              {entry.serviceName || 'Open service'}
            </Link>
          ) : (
            <span>{entry.reference}</span>
          )}
          {entry.relationship === 'SavedReference' && (
            <p className="text-xs text-muted-foreground">Configured use; connection not confirmed.</p>
          )}
          {entry.relationship === 'ServiceNetwork' && (
            <p className="text-xs text-muted-foreground">
              Service network configuration. Task coverage may be incomplete.
            </p>
          )}
        </li>
      ))}
    </ul>
  );
}
