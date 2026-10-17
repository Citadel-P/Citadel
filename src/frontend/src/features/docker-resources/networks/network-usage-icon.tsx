import { Network } from 'lucide-react';
import { getStateIconAppearance } from '@/components/custom/state-indicator';
import { cn } from '@/lib/utils';

export function NetworkUsageIcon({
  inUse,
  isStale,
  className,
}: {
  inUse?: boolean;
  isStale?: boolean;
  className?: string;
}) {
  const appearance = getStateIconAppearance(isStale ? 'Unknown' : (inUse ?? 'Unknown'), 'container');
  const label = isStale ? 'State is out of date' : appearance.label;
  return (
    <span className="inline-flex shrink-0" role="img" aria-label={label} title={label}>
      <Network className={cn(className, appearance.className)} />
    </span>
  );
}
