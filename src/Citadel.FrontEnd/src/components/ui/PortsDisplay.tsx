import { HostPortBinding } from '@/api/generated/api.types';
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import { EthernetPort, Link } from 'lucide-react';

interface Props {
  ports: Record<string, HostPortBinding[]>;
}

export function PortsDisplay({ ports }: Props) {
  return (
    <div className="flex flex-wrap gap-2">
      {ports &&
        Object.entries(ports).map(([containerPort, bindings]) => {
          // pick the first valid hostPort for main display
          const hostPort = bindings.find((b) => b.hostPort)?.hostPort;
          if (!hostPort) return null;
          const url = hostPort.endsWith('443') ? `https://localhost:${hostPort}` : `http://localhost:${hostPort}`;

          return (
            <HoverCard key={containerPort} openDelay={150} closeDelay={150}>
              <HoverCardTrigger>
                <div className="flex flex-row items-center justify-center cursor-pointer text-[13px] gap-2 hover:underline">
                  <EthernetPort width={14} height={14} className="text-primary" />{' '}
                  <span onClick={() => window.open(url, '_blank')}>{hostPort}</span>
                </div>
              </HoverCardTrigger>
              <HoverCardContent className="flex flex-col gap-3 p-3 text-xs bg-background w-fit">
                <div
                  className="flex items-center justify-center gap-1 hover:underline cursor-pointer"
                  onClick={() => window.open(url, '_blank')}>
                  <Link width={12} height={10} /> {url}
                </div>
                {bindings.map((b) => (
                  <span className="text-foreground/75">
                    - {b.hostIP ?? 'unknown'}:{b.hostPort}:{containerPort.toLowerCase()}
                  </span>
                ))}
              </HoverCardContent>
            </HoverCard>
          );
        })}
    </div>
  );
}
