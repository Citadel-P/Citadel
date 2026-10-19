import { RegistryView } from '@/api/generated/api.types';
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import { Link } from 'lucide-react';

export function RegistryDisplay({
  registry,
}: {
  registry: Pick<RegistryView, 'name' | 'registryHost' | 'type'> | undefined;
}) {
  if (!registry) return <></>;
  return (
    <div className="flex flex-wrap gap-2">
      {registry.name == '' ? (
        <span className="text-sm text-muted-foreground">&lt;unknown&gt;</span>
      ) : (
        <HoverCard openDelay={150} closeDelay={150}>
          <HoverCardTrigger>
            <div className="flex flex-row items-center justify-center cursor-pointer text-sm gap-2 hover:underline">
              <span>{registry.name}</span>
            </div>
          </HoverCardTrigger>
          <HoverCardContent className="flex flex-col gap-3 p-3 text-sm bg-background w-fit">
            <a
              className="flex items-center justify-center gap-1 hover:underline"
              href={registry.registryHost === '' ? 'https://hub.docker.com' : 'https://' + registry.registryHost}
              target="_blank"
              rel="noreferrer">
              <Link width={12} height={10} />{' '}
              {registry.registryHost === '' ? 'https://hub.docker.com' : 'https://' + registry.registryHost}
            </a>
            <span className="text-foreground/75">Provider: {registry.type}</span>
          </HoverCardContent>
        </HoverCard>
      )}
    </div>
  );
}
