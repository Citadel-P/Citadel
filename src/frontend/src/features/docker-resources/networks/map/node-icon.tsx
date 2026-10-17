import { CitadelIcons } from '@/lib/icons';
import type { MapNode } from './model';
import { getStateIconAppearance } from '@/components/custom/state-indicator';
import { cn } from '@/lib/utils';
import { NetworkUsageIcon } from '../network-usage-icon';

export function MapNodeIcon({ node, className }: { node: MapNode; className?: string }) {
  const Icon =
    node.kind === 'stack'
      ? CitadelIcons.Stack
      : node.kind === 'service'
        ? CitadelIcons.SwarmService
        : CitadelIcons[node.entries[0]?.workloadKind ?? 'Container'];
  if (node.kind === 'network') {
    return <NetworkUsageIcon inUse={node.inUse} isStale={node.isStale} className={className} />;
  }

  const entry = node.entries[0];
  const stale = node.isStale || entry?.isStale;
  const isContainer = node.kind === 'container';
  const status = stale ? 'Unknown' : ((isContainer ? entry?.containerState : entry?.workloadStatus) ?? 'Unknown');
  const appearance = getStateIconAppearance(status, isContainer ? 'container' : undefined);
  const label = stale ? 'State is out of date' : appearance.label;
  return (
    <span className="inline-flex shrink-0" role="img" aria-label={label} title={label}>
      <Icon className={cn(className, appearance.className)} />
    </span>
  );
}
