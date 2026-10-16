import { useEffect, useId, useRef, useState, type ReactNode } from 'react';
import { Activity } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Popover, PopoverAnchor, PopoverContent } from '@/components/ui/popover';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from '@/components/ui/dialog';
import { cn } from '@/lib/utils';

type HealthCardProps = {
  title: string;
  resourceName: string;
  triggerLabel: string;
  triggerClassName?: string;
  trigger: ReactNode;
  status: ReactNode;
  summary: ReactNode;
  renderDetails?: () => ReactNode;
  initiallyOpen?: boolean;
};

/** Shared health presentation; each resource supplies its evidence and authorized details. */
export function HealthCard({
  title,
  resourceName,
  triggerLabel,
  triggerClassName,
  trigger: triggerContent,
  status,
  summary,
  renderDetails,
  initiallyOpen = false,
}: HealthCardProps) {
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [open, setOpen] = useState(initiallyOpen);
  const pinned = useRef(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const closeTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const skipFocusOpen = useRef(false);
  const cardId = useId();
  const titleId = useId();
  const cancelClose = () => clearTimeout(closeTimer.current);
  useEffect(() => () => clearTimeout(closeTimer.current), []);

  const closeAfterLeave = () => {
    cancelClose();
    closeTimer.current = setTimeout(() => {
      if (
        !pinned.current &&
        !trigger.current?.matches(':focus') &&
        !content.current?.contains(document.activeElement)
      ) {
        setOpen(false);
      }
    }, 200);
  };

  const changeOpen = (value: boolean) => {
    cancelClose();
    pinned.current = false;
    setOpen(value);
  };

  return (
    <>
      <Popover open={open} onOpenChange={changeOpen}>
        <PopoverAnchor asChild>
          <Button
            ref={trigger}
            type="button"
            variant="ghost"
            size="sm"
            className={cn(
              'h-auto gap-1.5 rounded-sm p-0 hover:bg-transparent dark:hover:bg-transparent',
              triggerClassName,
            )}
            aria-label={triggerLabel}
            aria-haspopup="dialog"
            aria-expanded={open}
            aria-controls={open ? cardId : undefined}
            onPointerEnter={(event) => {
              if (event.pointerType === 'touch') return;
              cancelClose();
              setOpen(true);
            }}
            onPointerLeave={closeAfterLeave}
            onFocus={() => {
              if (!skipFocusOpen.current && !detailsOpen) setOpen(true);
              skipFocusOpen.current = false;
            }}
            onBlur={closeAfterLeave}
            onClick={() => {
              cancelClose();
              const next = !pinned.current;
              pinned.current = next;
              setOpen(next);
            }}
            onKeyDown={(event) => {
              if (event.key === 'ArrowDown' || (event.key === 'Tab' && !event.shiftKey && open)) {
                event.preventDefault();
                content.current?.focus();
              }
            }}>
            {triggerContent}
          </Button>
        </PopoverAnchor>
        <PopoverContent
          ref={content}
          id={cardId}
          aria-labelledby={titleId}
          tabIndex={-1}
          align="end"
          sideOffset={8}
          collisionPadding={12}
          className="max-h-[min(32rem,var(--radix-popover-content-available-height))] w-[min(26rem,calc(100vw-1.5rem))] overflow-y-auto overscroll-contain rounded-xl p-0 shadow-lg"
          onOpenAutoFocus={(event) => event.preventDefault()}
          onCloseAutoFocus={(event) => event.preventDefault()}
          onPointerEnter={cancelClose}
          onPointerLeave={closeAfterLeave}
          onFocus={cancelClose}
          onBlur={closeAfterLeave}
          onEscapeKeyDown={() => {
            if (document.activeElement !== trigger.current) {
              skipFocusOpen.current = true;
              trigger.current?.focus();
            }
          }}
          onInteractOutside={(event) => {
            if (event.target instanceof Node && trigger.current?.contains(event.target)) event.preventDefault();
          }}>
          <div className="flex items-center gap-3 border-b bg-muted/20 px-4 py-3">
            <h2 id={titleId} className="text-sm font-semibold">
              {title}
            </h2>
            <span className="ml-auto">{status}</span>
          </div>
          <div className="p-4">{summary}</div>
          {renderDetails && (
            <div className="border-t px-4 py-3">
              <Button
                variant="outline"
                size="sm"
                className="w-full"
                onClick={() => {
                  changeOpen(false);
                  setDetailsOpen(true);
                }}>
                View details
              </Button>
            </div>
          )}
        </PopoverContent>
      </Popover>
      {renderDetails && (
        <Dialog open={detailsOpen} onOpenChange={setDetailsOpen}>
          <DialogContent
            className="flex max-h-[calc(100dvh-2rem)] flex-col gap-0 overflow-hidden p-0 sm:max-w-3xl"
            onCloseAutoFocus={(event) => {
              event.preventDefault();
              skipFocusOpen.current = true;
              trigger.current?.focus();
            }}>
            <DialogHeader className="shrink-0 flex-row items-center gap-3 border-b px-4 py-4 pr-12 text-left sm:px-6 sm:pr-12">
              <span className="flex size-10 shrink-0 items-center justify-center rounded-lg border bg-muted/40 text-muted-foreground">
                <Activity className="size-5" aria-hidden="true" />
              </span>
              <div className="min-w-0 space-y-1">
                <DialogTitle className="text-base">Health details</DialogTitle>
                <DialogDescription className="break-words text-xs">{resourceName}</DialogDescription>
              </div>
            </DialogHeader>
            <div className="min-h-0 flex-1 overflow-y-auto p-4 sm:p-6">{detailsOpen && renderDetails()}</div>
          </DialogContent>
        </Dialog>
      )}
    </>
  );
}
