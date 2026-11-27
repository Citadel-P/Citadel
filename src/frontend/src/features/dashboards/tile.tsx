import { createContext, useCallback, useContext, type CSSProperties, type ReactNode } from 'react';
import { CircleAlert, Cpu, MemoryStick, Network, HardDrive } from 'lucide-react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Skeleton } from '@/components/ui/skeleton';
import { cn } from '@/lib/utils';
import { Surface } from '@/components/custom/surface';
import { dashboardGrid } from './layout';
import { chartPalette } from './display-controls';
import './widgets.css';
import { CitadelIcons } from '@/lib/icons';
import { catalogue } from './catalogue';
import { TileSlots, useTileSlots } from './tile-slots';
import { createPortal } from 'react-dom';
import type { DashboardWidget } from '@/api/generated/api.types';

export const ClearScopeContext = createContext<(() => void) | undefined>(undefined);
export const ViewOnlyContext = createContext(false);
export function WidgetNotice({ children }: { children: ReactNode }) {
  return (
    <Alert className="px-3 py-2" role="status">
      <CircleAlert />
      <AlertDescription>{children}</AlertDescription>
    </Alert>
  );
}

export function WidgetSkeleton({ kind }: { kind?: DashboardWidget['settings']['kind'] }) {
  return (
    <div role="status" className="flex min-h-0 flex-1 flex-col justify-center gap-3">
      <span className="sr-only">Loading…</span>
      {kind === 'FleetSummary' ? (
        <div className="flex items-center justify-around gap-3">
          {[1, 2, 3].map((i) => (
            <Skeleton key={i} className="h-8 w-1/5" />
          ))}
        </div>
      ) : kind === 'Metrics' || kind === 'ResourceState' ? (
        <>
          <Skeleton className="h-6 w-24" />
          <Skeleton className={kind === 'Metrics' ? 'min-h-8 flex-1' : 'h-2 w-full'} />
          <Skeleton className="h-3 w-1/3" />
        </>
      ) : (
        [1, 2].map((i) => <Skeleton key={i} className="h-6 w-full" />)
      )}
    </div>
  );
}

export function Tile({
  tile,
  children,

  controls,
  pending = false,
}: {
  tile: DashboardWidget;
  children: ReactNode;
  pending?: boolean;
  controls?: ReactNode;
}) {
  const { x, y, w, h } = tile.layout;
  const target = useContext(TileSlots)?.slots.get(tile.id);

  const settings = tile.settings;
  const Icon =
    settings.kind === 'Metrics'
      ? { Cpu, Memory: MemoryStick, Network, DiskIo: HardDrive }[settings.metric]
      : settings.kind === 'ResourceState'
        ? CitadelIcons[settings.resourceKind]
        : catalogue[settings.kind].Icon;
  const summary = tile.settings.kind === 'FleetSummary';
  const section = (
    <Surface
      aria-label={tile.title}
      aria-busy={pending || undefined}
      data-widget={settings.kind}
      data-roomy={h >= 5}
      className={
        'dashboard-card flex h-full min-h-0 min-w-0 flex-col overflow-hidden p-0 md:[grid-column:var(--tile-column)] md:[grid-row:var(--tile-row)]' +
        (summary ? ' md:flex-row md:items-center' : '')
      }
      style={
        {
          '--tile-column': x + 1 + ' / span ' + w,
          '--tile-row': y + 1 + ' / span ' + h,
          '--widget-ink': settings.kind === 'Metrics' ? chartPalette[settings.color] : undefined,
          '--arrival-delay': Math.min(y, 5) * 22 + 'ms',
        } as CSSProperties
      }>
      <div className="dashboard-card-head flex shrink-0 flex-wrap items-center justify-between gap-1 px-4 pt-3">
        <h2 className="flex min-w-0 items-center gap-2 text-base font-semibold">
          <span className="dashboard-card-symbol">
            <Icon aria-hidden="true" className="size-4" />
          </span>
          <span className="truncate" title={tile.title}>
            {tile.title}
          </span>
        </h2>
        {controls}
      </div>
      <div
        className={cn(
          'dashboard-widget-scroll flex min-h-0 flex-1 flex-col gap-2 overflow-auto px-4 py-2 text-sm',
          tile.settings.kind === 'ResourceState' && h <= 3 && 'gap-1 py-1',
        )}>
        {children}
      </div>
    </Surface>
  );
  return target ? createPortal(section, target) : null;
}

export function Slot({
  id,
  register,
  editor,
}: {
  id: string;
  editor?: boolean;
  register: (id: string, node: HTMLDivElement | null) => void;
}) {
  const attach = useCallback((node: HTMLDivElement | null) => register(id, node), [id, register]);
  return (
    <div
      aria-hidden={editor || undefined}
      className={editor ? 'h-full min-h-0' : 'contents'}
      inert={editor ? true : undefined}
      ref={attach}
    />
  );
}

export function OrderedTiles({ tiles, children }: { tiles: DashboardWidget[]; children: ReactNode }) {
  const { target, register } = useTileSlots();
  return (
    <div
      className="grid md:grid-cols-12 md:auto-rows-(--dashboard-row-height)"
      style={{ gap: dashboardGrid.gap, '--dashboard-row-height': dashboardGrid.rowHeight + 'px' } as CSSProperties}>
      {tiles.map((t) => (
        <Slot key={t.id} id={t.id} register={register} />
      ))}
      <TileSlots.Provider value={target}>{children}</TileSlots.Provider>
    </div>
  );
}
