import { useEffect, useRef, useState } from 'react';
import {
  GridLayout,
  calcGridItemPosition,
  useContainerWidth,
  cloneLayout,
  collides,
  verticalCompactor,
  type Layout,
  type LayoutItem,
} from 'react-grid-layout';
import 'react-grid-layout/css/styles.css';
import './editor.css';
import 'react-resizable/css/styles.css';
import { dashboardGrid } from './layout';
import { OverviewTiles } from './overview';
import type { DashboardRequirements } from './requirements';
import { ReadAdmissionContext, createReadAdmission } from '@/lib/read-admission';
import { TileSlots, useTileSlots } from './tile-slots';
import { Slot, ViewOnlyContext } from './tile';
import { GripVertical } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { DashboardWidget } from '@/api/generated/api.types';
import { catalogue, widgetSourceError, type WidgetKind } from './catalogue';

const gap = dashboardGrid.gap;
const spacing: [number, number] = [gap, gap];
const gridConfig = {
  cols: 12,
  rowHeight: dashboardGrid.rowHeight,
  margin: spacing,
  containerPadding: spacing,
  maxRows: 536,
};

// RGL handles collisions. Keep gaps in already-valid layouts, including initial mount.
export const dashboardCompactor: typeof verticalCompactor = {
  ...verticalCompactor,
  compact(layout, cols) {
    const overlap = layout.some((item, index) => layout.slice(0, index).some((other) => collides(item, other)));
    return overlap ? verticalCompactor.compact(layout, cols) : cloneLayout(layout);
  },
};

export function applyGeometry(widgets: DashboardWidget[], layout: Layout) {
  return widgets.map((widget) => {
    const pos = layout.find((p) => p.i === widget.id);
    return pos ? { ...widget, layout: { x: pos.x, y: pos.y, w: pos.w, h: pos.h } } : widget;
  });
}
export function EditorGrid({
  widgets,
  tags,
  minutes,
  valid,
  plan,
  selected,
  select,
  onMove,
  dragKind,
  setDragKind,
  drop,
}: {
  widgets: DashboardWidget[];
  tags: string[];
  minutes: number;
  valid: DashboardWidget[];
  plan: DashboardRequirements;
  selected?: string;
  select: (id: string) => void;
  onMove: (layout: Layout) => void;
  dragKind?: WidgetKind;
  setDragKind: (kind?: WidgetKind) => void;
  drop: (kind: WidgetKind, layout: Layout, item: LayoutItem) => void;
}) {
  const { width, containerRef } = useContainerWidth({ initialWidth: 720 });
  const [epoch, setEpoch] = useState(0);
  const [gesture, setGesture] = useState<LayoutItem>();
  const [announcement, announce] = useState('');
  const active = useRef(false);
  const dragged = useRef(false);
  useEffect(() => {
    const cancel = () => {
      if (!active.current && !dragKind) return;
      active.current = false;
      setGesture(undefined);
      setDragKind(undefined);
      setEpoch((v) => v + 1);
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') cancel();
    };
    window.addEventListener('keydown', key);
    window.addEventListener('pointercancel', cancel);
    return () => {
      window.removeEventListener('keydown', key);
      window.removeEventListener('pointercancel', cancel);
    };
  }, [dragKind, setDragKind]);
  const { target, register } = useTileSlots(true);
  const [admit] = useState(() => createReadAdmission(2));
  const stacked = width < 480;
  const ordered = [...widgets].sort((a, b) => a.layout.y - b.layout.y || a.layout.x - b.layout.x);
  // On narrow canvases the inspector still edits the saved twelve-column layout.
  let nextRow = 0;
  const layout: LayoutItem[] = [];
  for (const w of ordered) {
    const position = stacked ? { ...w.layout, x: 0, y: nextRow, w: 12 } : w.layout;
    nextRow += w.layout.h;
    layout.push({
      i: w.id,
      ...position,
      minW: catalogue[w.settings.kind].minW,
      minH: catalogue[w.settings.kind].minH,
      maxH: 24,
    });
  }
  const size = catalogue[dragKind ?? 'Notes'].size;
  const ghost = dragKind ? catalogue[dragKind].label + ' · ' + size.w + ' × ' + size.h : '';
  const move = (next: Layout, _previous: LayoutItem | null, item: LayoutItem | null) => {
    if (active.current && next.every((p) => p.y <= 512)) {
      onMove(next);
      const placed = next.find((p) => p.i === item?.i);
      if (placed) {
        const title = widgets.find((w) => w.id === placed.i)?.title ?? 'Widget';
        announce(`${title} at column ${placed.x + 1}, row ${placed.y + 1}, ${placed.w} × ${placed.h}`);
      }
    }
    active.current = false;
    setGesture(undefined);
  };
  const preview = (item: LayoutItem | null) => {
    if (item)
      setGesture((prev) =>
        prev?.i === item.i && prev.x === item.x && prev.y === item.y && prev.w === item.w && prev.h === item.h
          ? prev
          : { ...item },
      );
  };
  const destination = gesture && (gesture.i !== '__drop__' || dragKind) ? gesture : undefined;
  const footprint =
    destination &&
    calcGridItemPosition(
      { ...gridConfig, containerWidth: width },
      destination.x,
      destination.y,
      destination.w,
      destination.h,
    );
  const positionLabel =
    destination && `Column ${destination.x + 1}, row ${destination.y + 1} · ${destination.w} × ${destination.h}`;
  return (
    <div
      ref={containerRef}
      className="dashboard-placement min-w-0"
      data-guides={!stacked}
      data-moving={!!(destination || ghost)}
      style={
        {
          '--grid-gap': gap + 'px',
          '--grid-column-step': (width - gap) / gridConfig.cols + 'px',
          '--grid-row-step': dashboardGrid.rowHeight + dashboardGrid.gap + 'px',
        } as React.CSSProperties
      }>
      {!stacked && (
        <div className="dashboard-column-ruler" aria-hidden="true">
          {Array.from({ length: 12 }, (_, i) => (
            <span key={i} data-covered={destination && i >= destination.x && i < destination.x + destination.w}>
              {i + 1}
            </span>
          ))}
        </div>
      )}
      <p className="sr-only" aria-live="polite">
        {positionLabel || ghost}
      </p>
      <span className="sr-only" role="status">
        {announcement}
      </span>
      <GridLayout
        className="dashboard-editor-grid"
        key={epoch}
        width={width}
        layout={layout}
        compactor={dashboardCompactor}
        gridConfig={gridConfig}
        style={
          {
            minHeight: 240,
            '--dashboard-ghost': JSON.stringify(ghost),
            ...(footprint && {
              '--drop-left': footprint.left + 'px',
              '--drop-top': footprint.top + 'px',
              '--drop-width': footprint.width + 'px',
              '--drop-height': footprint.height + 'px',
            }),
          } as React.CSSProperties
        }
        resizeConfig={{ enabled: !stacked }}
        dragConfig={{ enabled: !stacked, handle: '.dashboard-drag-handle' }}
        dropConfig={{
          enabled: !stacked,
          defaultItem: size,
          onDragOver: (e) => (e.dataTransfer?.types.includes('application/x-citadel-widget') ? size : false),
        }}
        droppingItem={{ i: '__drop__', x: 0, y: 0, ...size }}
        onDrop={(next, item, event) => {
          const kind =
            (event instanceof DragEvent && event.dataTransfer?.getData('application/x-citadel-widget')) || dragKind;
          if (kind && Object.hasOwn(catalogue, kind) && item && next.every((p) => p.y <= 512))
            drop(kind as WidgetKind, next, item);
          active.current = false;
          setGesture(undefined);
          setDragKind(undefined);
        }}
        onDragStart={(_, __, item) => {
          dragged.current = true;
          active.current = true;
          preview(item);
        }}
        onResizeStart={(_, __, item) => {
          active.current = true;
          preview(item);
        }}
        onDrag={(_, __, item) => preview(item)}
        onResize={(_, __, item, placeholder) => preview(placeholder ?? item)}
        onDragStop={move}
        onResizeStop={move}>
        {ordered.map((w) => {
          const Icon = catalogue[w.settings.kind].Icon;
          const sourceError = widgetSourceError(w.settings);
          return (
            <div
              key={w.id}
              id={'tile-' + w.id}
              data-selected={selected === w.id}
              className="dashboard-edit-tile group/tile flex min-w-0 flex-col">
              <Button
                type="button"
                variant="ghost"
                aria-label={w.title || catalogue[w.settings.kind].label}
                aria-pressed={selected === w.id}
                className="dashboard-drag-handle absolute inset-0 z-1 h-full w-full rounded-md hover:bg-transparent dark:hover:bg-transparent focus-visible:ring-ring"
                onPointerDown={() => {
                  dragged.current = false;
                }}
                onClick={(e) => {
                  if (e.detail === 0 || !dragged.current) select(w.id);
                }}
              />
              {(selected === w.id || gesture?.i === w.id) && (
                <span aria-hidden="true" className="dashboard-selection-size">
                  {gesture?.i === w.id ? positionLabel : `${w.layout.w} × ${w.layout.h}`}
                </span>
              )}
              {!stacked && (
                <span
                  title={'Move ' + w.title}
                  aria-hidden="true"
                  className="dashboard-grip pointer-events-none absolute right-1 top-1 z-2 rounded bg-card p-1 text-muted-foreground opacity-0 group-hover/tile:opacity-100 group-focus-within/tile:opacity-100">
                  <GripVertical className="size-4" />
                </span>
              )}
              {sourceError ? (
                <div className="m-3 flex min-h-0 flex-1 flex-col items-center justify-center gap-2 rounded-md border border-dashed bg-muted/15 p-2 text-center text-xs text-muted-foreground">
                  <Icon className="size-5" />
                  {sourceError}
                </div>
              ) : (
                <Slot id={w.id} register={register} editor />
              )}
            </div>
          );
        })}
      </GridLayout>
      {valid.length > 0 && (
        <ReadAdmissionContext.Provider value={admit}>
          <ViewOnlyContext.Provider value={true}>
            <TileSlots.Provider value={target}>
              <OverviewTiles plan={plan} tags={tags} range={minutes} />
            </TileSlots.Provider>
          </ViewOnlyContext.Provider>
        </ReadAdmissionContext.Provider>
      )}
    </div>
  );
}
