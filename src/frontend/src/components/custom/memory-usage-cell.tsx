import { useEffect, useId, useRef, useState } from 'react';
import { ContainerStateStatus, type ContainerStatView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Popover, PopoverAnchor, PopoverContent } from '@/components/ui/popover';
import { byteTransform } from '@/lib/bytes.helper';
import { memoryBreakdown } from '@/lib/memory-stats';
import { cn } from '@/lib/utils';

export function MemoryUsageCell({
  state,
  stats,
  className,
}: {
  state: ContainerStateStatus;
  stats?: Partial<ContainerStatView> | null;
  className?: string;
}) {
  const [open, setOpen] = useState(false);
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

  if (state !== ContainerStateStatus.Running) {
    return <div className={cn('text-muted-foreground', className)}>0B / 0B</div>;
  }
  const breakdown = stats ? memoryBreakdown(stats) : undefined;
  const format = (value: number | null | undefined) =>
    value != null && Number.isFinite(value) && value >= 0 ? byteTransform(value, 2) : '—';
  const usage = `${format(stats?.memoryActive)} / ${format(stats?.memoryLimit)}`;
  const rows = [
    ['Memory excluding file cache', breakdown?.nonCache, 'bg-chart-1'],
    ['Active file cache', breakdown?.activeCache, 'bg-chart-2'],
    ['Inactive file cache', stats?.memoryCache, 'bg-chart-3'],
  ] as const;
  return (
    <Popover open={open} onOpenChange={changeOpen}>
      <PopoverAnchor asChild>
        <Button
          ref={trigger}
          type="button"
          variant="ghost"
          className={cn(
            'h-auto rounded-sm p-0 text-[13px] font-normal tabular-nums hover:bg-transparent dark:hover:bg-transparent',
            className,
          )}
          aria-label={`Memory breakdown: ${usage}`}
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
            if (!skipFocusOpen.current) setOpen(true);
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
          {usage}
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
        <div className="border-b bg-muted/20 px-4 py-3">
          <h2 id={titleId} className="text-sm font-semibold">
            Memory breakdown
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">Working set includes active file cache.</p>
        </div>
        <div className="space-y-3 p-4 text-sm">
          <dl className="space-y-2">
            <div className="flex justify-between gap-4 font-medium">
              <dt>Working set</dt>
              <dd className="tabular-nums">{format(stats?.memoryActive)}</dd>
            </div>
            {rows.map(([label, value, color]) => (
              <div key={label} className="flex items-center justify-between gap-4">
                <dt className="flex items-center gap-2">
                  <span aria-hidden="true" className={cn('size-2 shrink-0 rounded-sm', color)} />
                  {label}
                </dt>
                <dd className="tabular-nums">{format(value)}</dd>
              </div>
            ))}
            <div className="flex justify-between gap-4 border-t pt-2">
              <dt>Memory limit</dt>
              <dd className="tabular-nums">{format(stats?.memoryLimit)}</dd>
            </div>
          </dl>
          {breakdown?.nonCache == null && (
            <p className="text-xs text-muted-foreground">The full breakdown is unavailable for this reading.</p>
          )}
          <p className="text-xs text-muted-foreground">
            Shared memory remains in memory excluding file cache. This is not process RSS.
          </p>
        </div>
      </PopoverContent>
    </Popover>
  );
}
