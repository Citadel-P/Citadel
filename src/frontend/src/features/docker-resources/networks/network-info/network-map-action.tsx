import { useRef, useState } from 'react';
import { useParams } from 'react-router';
import { Waypoints } from 'lucide-react';
import type { NetworkView } from '@/api/generated/api.types';
import { TabActions } from '@/components/custom/tab-actions';
import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { hasCapability } from '@/lib/resource-capabilities';
import { NetworkDetailsSheet } from '../network-details-sheet';

export function NetworkMapAction({ network }: { network: NetworkView }) {
  const { platformId } = useParams<{ platformId: string }>();
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  if (!platformId || !hasCapability(network, 'canInspect')) return null;

  return (
    <>
      <TabActions>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              ref={trigger}
              type="button"
              variant="ghost"
              size="icon"
              className="size-8 shrink-0 text-muted-foreground"
              aria-label="View connections"
              aria-haspopup="dialog"
              aria-expanded={open}
              onClick={() => setOpen(true)}>
              <Waypoints className="size-4" aria-hidden="true" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>View connections</TooltipContent>
        </Tooltip>
      </TabActions>
      {open && (
        <NetworkDetailsSheet
          network={network}
          networks={[network]}
          platformId={platformId}
          context="details"
          onClose={() => setOpen(false)}
          onCloseAutoFocus={(event) => {
            event.preventDefault();
            trigger.current?.focus();
          }}
        />
      )}
    </>
  );
}
