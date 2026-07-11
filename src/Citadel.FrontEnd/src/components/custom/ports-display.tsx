import { HostPortBinding } from '@/api/generated/api.types';
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import { cn } from '@/lib/utils';
import { EthernetPort, Link } from 'lucide-react';
import { OverflowCountBadge } from './common';

interface Props {
  ports?: Record<string, HostPortBinding[]> | null;
  compact?: boolean;
  maxVisible?: number;
  className?: string;
}

type PublishedPortView = {
  containerPort: string;
  bindings: HostPortBinding[];
  hostPort: string;
  url: string;
};

export function PortsDisplay({
  ports,
  compact = false,
  maxVisible = compact ? 1 : Number.POSITIVE_INFINITY,
  className,
}: Props) {
  const visibleBindings = getPublishedPorts(ports);
  const displayBindings = visibleBindings.slice(0, maxVisible);
  const hiddenCount = Math.max(visibleBindings.length - displayBindings.length, 0);

  if (visibleBindings.length === 0) return null;

  return (
    <HoverCard openDelay={150} closeDelay={150}>
      <HoverCardTrigger asChild>
        <div
          className={cn(
            'inline-flex max-w-full cursor-pointer items-center gap-2',
            compact ? 'whitespace-nowrap' : 'flex-wrap',
            className,
          )}>
          {displayBindings.map((port) => (
            <button
              key={`${port.containerPort}:${port.hostPort}`}
              type="button"
              className="inline-flex min-w-0 items-center gap-1.5 hover:underline"
              onClick={() => window.open(port.url, '_blank')}>
              <EthernetPort width={14} height={14} className="shrink-0 text-primary" />
              <span className="truncate">{port.hostPort}</span>
            </button>
          ))}
          <OverflowCountBadge count={hiddenCount} title={`${hiddenCount} more ports`} />
        </div>
      </HoverCardTrigger>
      <HoverCardContent className="flex w-fit max-w-100 flex-col gap-3 bg-background p-3 text-xs">
        {visibleBindings.map((port) => (
          <div key={`${port.containerPort}:${port.hostPort}`} className="flex flex-col gap-1">
            <button
              type="button"
              className="flex items-center gap-1 text-left hover:underline"
              onClick={() => window.open(port.url, '_blank')}>
              <Link width={12} height={10} className="shrink-0" /> {port.url}
            </button>
            {port.bindings.map((binding) => (
              <span
                key={`${port.containerPort}:${binding.hostPort ?? ''}:${binding.hostIP ?? ''}`}
                className="text-foreground/75">
                - {binding.hostIP ?? ':'}:{binding.hostPort}:{port.containerPort.toLowerCase()}
              </span>
            ))}
          </div>
        ))}
      </HoverCardContent>
    </HoverCard>
  );
}

const getPublishedPorts = (ports?: Record<string, HostPortBinding[]> | null): PublishedPortView[] => {
  if (!ports) return [];

  return Object.entries(ports).flatMap(([containerPort, bindings]) => {
    const publishedBindings = bindings.filter((binding) => binding.hostPort);
    const hostPort = publishedBindings[0]?.hostPort;
    if (!hostPort) return [];

    return {
      containerPort,
      bindings: publishedBindings,
      hostPort,
      url: hostPort.endsWith('443') ? `https://localhost:${hostPort}` : `http://localhost:${hostPort}`,
    };
  });
};
