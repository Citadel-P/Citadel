import {
  createContext,
  useCallback,
  useContext,
  useLayoutEffect,
  useState,
  type CSSProperties,
  type ReactNode,
} from 'react';
import { CircleAlert } from 'lucide-react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Surface } from '@/components/custom/surface';
import { dashboardGrid } from './layout';
import { catalogue } from './catalogue';
import { createPortal } from 'react-dom';
import type { DashboardWidget } from '@/api/generated/api.types';

export const CaptureContext = createContext<((key: string, value: ReactNode, pending: boolean) => void) | undefined>(
  undefined,
);
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

export function Tile({
  tile,
  children,
  pending = false,
}: {
  tile: DashboardWidget;
  children: ReactNode;
  pending?: boolean;
}) {
  const capture = useContext(CaptureContext);
  useLayoutEffect(() => capture?.(tile.id, children, pending), [capture, tile.id, children, pending]);
  const { x, y, w, h } = tile.layout;
  const slots = useContext(TileSlots);
  const editor = useContext(EditorTileSlots);
  if (editor) {
    const target = editor.get(tile.id);
    return target ? createPortal(children, target) : null;
  }
  const Icon = catalogue[tile.settings.kind].Icon;
  const summary = tile.settings.kind === 'FleetSummary';
  const section = (
    <Surface
      aria-label={tile.title}
      className={
        'flex min-h-0 min-w-0 flex-col overflow-hidden p-0 md:[grid-column:var(--tile-column)] md:[grid-row:var(--tile-row)]' +
        (summary ? ' md:flex-row md:items-center' : '')
      }
      style={{ '--tile-column': x + 1 + ' / span ' + w, '--tile-row': y + 1 + ' / span ' + h } as CSSProperties}>
      <h2
        className={'flex shrink-0 items-center gap-2 px-4 py-3 text-base font-semibold ' + (summary ? '' : 'border-b')}>
        <Icon className="size-4 shrink-0 text-muted-foreground" />
        <span className="truncate" title={tile.title}>
          {tile.title}
        </span>
      </h2>
      <div className="dashboard-widget-scroll flex min-h-0 flex-1 flex-col gap-2 overflow-auto px-4 py-2 text-sm">
        {children}
      </div>
    </Surface>
  );
  return slots ? (slots.get(tile.id) ? createPortal(section, slots.get(tile.id)!) : null) : section;
}
const TileSlots = createContext<Map<string, HTMLDivElement> | undefined>(undefined);

export const EditorTileSlots = createContext<Map<string, HTMLDivElement> | undefined>(undefined);

export function useFrozenDashboard() {
  const paused = useContext(ViewOnlyContext);
  const editing = useContext(EditorTileSlots);
  return paused && !editing;
}
export function Slot({
  id,
  register,
  editor,
}: {
  id: string;
  editor?: string;
  register: (id: string, node: HTMLDivElement | null) => void;
}) {
  const attach = useCallback((node: HTMLDivElement | null) => register(id, node), [id, register]);
  return (
    <div
      role={editor ? 'region' : undefined}
      aria-label={editor ? editor + ' preview' : undefined}
      className={
        editor
          ? 'dashboard-widget-scroll flex min-h-0 flex-1 flex-col gap-2 overflow-auto px-3 py-2 text-sm'
          : 'contents'
      }
      ref={attach}
    />
  );
}

// Grouped data owners render into slots in the saved layout's reading order.
export function useTileSlots() {
  const [slots, setSlots] = useState(() => new Map<string, HTMLDivElement>());
  const register = useCallback((id: string, node: HTMLDivElement | null) => {
    setSlots((old) => {
      if ((old.get(id) ?? null) === node) return old;
      const next = new Map(old);
      if (node) next.set(id, node);
      else next.delete(id);
      return next;
    });
  }, []);
  return { slots, register };
}

export function OrderedTiles({ tiles, children }: { tiles: DashboardWidget[]; children: ReactNode }) {
  const { slots, register } = useTileSlots();
  if (useContext(EditorTileSlots)) return children;
  return (
    <div
      className="grid md:grid-cols-12 md:auto-rows-(--dashboard-row-height)"
      style={{ gap: dashboardGrid.gap, '--dashboard-row-height': dashboardGrid.rowHeight + 'px' } as CSSProperties}>
      {tiles.map((t) => (
        <Slot key={t.id} id={t.id} register={register} />
      ))}
      <TileSlots.Provider value={slots}>{children}</TileSlots.Provider>
    </div>
  );
}
