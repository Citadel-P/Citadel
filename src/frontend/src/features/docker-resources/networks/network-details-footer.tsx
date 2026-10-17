import { Link } from 'react-router';
import { ArrowUpRight } from 'lucide-react';
import type { NetworkView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { SheetFooter } from '@/components/ui/sheet';
import { hasCapability } from '@/lib/resource-capabilities';
import { networkDetailsPath } from './network-presentation';

export function NetworkDetailsFooter({ network, platformId }: { network?: NetworkView; platformId: string }) {
  if (!network || !hasCapability(network, 'canInspect')) return null;
  return (
    <SheetFooter className="shrink-0 border-t bg-background px-5 py-4">
      <Button asChild variant="outline" className="w-full">
        <Link to={networkDetailsPath(platformId, network)} title={network.name}>
          Open network details <ArrowUpRight className="size-4" />
        </Link>
      </Button>
    </SheetFooter>
  );
}
