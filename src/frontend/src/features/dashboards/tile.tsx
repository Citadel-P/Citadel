import { createContext, useContext, useLayoutEffect, type CSSProperties, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { useCallback, useState } from 'react';
import type { DashboardWidget } from '@/api/generated/api.types';

export const CaptureContext = createContext<((key: string, value: ReactNode, pending: boolean) => void) | undefined>(
  undefined,
);
export const ViewOnlyContext = createContext(false);
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
  const section = (
    <section
      aria-label={tile.title}
      className="flex min-w-0 flex-col gap-3 overflow-auto rounded-lg border bg-card p-4 md:[grid-column:var(--tile-column)] md:[grid-row:var(--tile-row)]"
      style={{ '--tile-column': x + 1 + ' / span ' + w, '--tile-row': y + 1 + ' / span ' + h } as CSSProperties}>
      <h2 className="text-sm font-medium">{tile.title}</h2>
      {children}
    </section>
  );
  return slots ? (slots.get(tile.id) ? createPortal(section, slots.get(tile.id)!) : null) : section;
}
const TileSlots = createContext<Map<string, HTMLDivElement> | undefined>(undefined);

// Data owners group identical reads; slots keep the document in the saved layout's reading order.
export function OrderedTiles({ tiles, children }: { tiles: DashboardWidget[]; children: ReactNode }) {
  const [slots] = useState(() => new Map<string, HTMLDivElement>());
  const [ready, setReady] = useState(false);
  const mounted = useCallback((node: HTMLDivElement | null) => setReady(!!node), []);
  return (
    <div ref={mounted} className="grid gap-4 md:grid-cols-12 md:auto-rows-[3.5rem]">
      {tiles.map((tile) => (
        <div
          className="contents"
          key={tile.id}
          ref={(node) => {
            if (node) slots.set(tile.id, node);
            else slots.delete(tile.id);
          }}
        />
      ))}
      {ready && <TileSlots.Provider value={slots}>{children}</TileSlots.Provider>}
    </div>
  );
}
