import { RegistryView } from '@/api/generated/api.types';
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import { Link } from 'lucide-react';

export function RegistryDisplay({ registry }: { registry: RegistryView | undefined }) {
  if (!registry) return <></>;
  return (
    <div className="flex flex-wrap gap-2">
      <HoverCard openDelay={150} closeDelay={150}>
        <HoverCardTrigger>
          <div className="flex flex-row items-center justify-center cursor-pointer text-[13px] gap-2 hover:underline">
            <span>{registry.name == '' ? 'Docker Hub' : registry.name}</span>
          </div>
        </HoverCardTrigger>
        <HoverCardContent className="flex flex-col gap-3 p-3 text-xs bg-background w-fit">
          <div
            className="flex items-center justify-center gap-1 hover:underline cursor-pointer"
            onClick={() => window.open(registry.url == '' ? 'https://hub.docker.com' : registry.url, '_blank')}>
            <Link width={12} height={10} /> {registry.url == '' ? 'https://hub.docker.com' : registry.url}
          </div>
          <span className="text-foreground/75">Provider: {registry.type}</span>
        </HoverCardContent>
      </HoverCard>
    </div>
  );
}
