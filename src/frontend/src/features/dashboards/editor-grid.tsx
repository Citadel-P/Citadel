import { useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import {
  GridLayout,
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
import { dashboardRequirements } from './requirements';
import { focusManager } from '@tanstack/react-query';
import { ReadAdmissionContext, createReadAdmission } from '@/lib/read-admission';
import { EditorTileSlots, Slot, useTileSlots, ViewOnlyContext } from './tile';
import { GripVertical } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { DashboardWidget } from '@/api/generated/api.types';
import { catalogue, widgetSourceError, type WidgetKind } from './catalogue';

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
  selected?: string;
  select: (id: string) => void;
  onMove: (layout: Layout) => void;
  dragKind?: WidgetKind;
  setDragKind: (kind?: WidgetKind) => void;
  drop: (kind: WidgetKind, layout: Layout, item: LayoutItem) => void;
}) {
  const { width, containerRef } = useContainerWidth({ initialWidth: 720 });
  const [epoch, setEpoch] = useState(0);
  const [gesture, setGesture] = useState('');
  const [announcement, announce] = useState('');
  const active = useRef(false);
  useEffect(() => {
    const cancel = () => {
      if (!active.current && !dragKind) return;
      active.current = false;
      setGesture('');
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
  const { slots, register } = useTileSlots();
  const [admit] = useState(() => createReadAdmission(2));
  const visible = useSyncExternalStore(focusManager.subscribe, () => focusManager.isFocused());
  const valid = useMemo(() => widgets.filter((w) => !widgetSourceError(w.settings)), [widgets]);
  const plan = useMemo(() => dashboardRequirements(valid, tags, minutes), [valid, tags, minutes]);
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
      if (item)
        announce(
          (widgets.find((w) => w.id === item.i)?.title ?? 'Widget') +
            ' at column ' +
            item.x +
            ', row ' +
            item.y +
            ', ' +
            item.w +
            ' × ' +
            item.h,
        );
    }
    active.current = false;
    setGesture('');
  };
  const preview = (item: LayoutItem | null) => {
    if (item) setGesture((widgets.find((w) => w.id === item.i)?.title ?? 'Widget') + ' · ' + item.w + ' × ' + item.h);
  };
  return (
    <div ref={containerRef} className="min-w-0">
      <p className="sr-only" aria-live="polite">
        {gesture || ghost}
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
        gridConfig={{
          cols: 12,
          rowHeight: dashboardGrid.rowHeight,
          margin: [dashboardGrid.gap, dashboardGrid.gap],
          maxRows: 536,
        }}
        style={{
          minHeight: 240,
          ...({
            '--dashboard-ghost': JSON.stringify(gesture || ghost),
          } as React.CSSProperties),
        }}
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
          setDragKind(undefined);
        }}
        onDragStart={() => {
          active.current = true;
        }}
        onResizeStart={() => {
          active.current = true;
        }}
        onDrag={(_, __, item) => preview(item)}
        onResize={(_, __, item) => preview(item)}
        onDragStop={move}
        onResizeStop={move}>
        {ordered.map((w) => {
          const Icon = catalogue[w.settings.kind].Icon;
          return (
            <div
              key={w.id}
              data-selected={selected === w.id}
              className="dashboard-edit-tile flex min-w-0 flex-col border bg-card">
              {selected === w.id && (
                <span className="pointer-events-none absolute -top-2.5 start-3 z-1 rounded bg-blue-600 px-1.5 text-[10px] font-semibold uppercase tracking-wide text-white">
                  Editing
                </span>
              )}
              <header className="flex shrink-0 items-center gap-2 border-b bg-muted/25 px-3 py-1.5">
                <GripVertical
                  aria-label={'Move ' + w.title}
                  className={
                    stacked
                      ? 'hidden'
                      : 'dashboard-drag-handle size-4 shrink-0 cursor-grab text-muted-foreground/70 active:cursor-grabbing'
                  }
                />
                <Icon className="size-4 shrink-0 text-muted-foreground" />
                <Button
                  variant="ghost"
                  size="sm"
                  aria-pressed={selected === w.id}
                  className="h-7 min-w-0 shrink justify-start overflow-hidden px-0.5 text-xs font-semibold uppercase tracking-wider"
                  onClick={() => select(w.id)}>
                  <span className="truncate">{w.title || catalogue[w.settings.kind].label}</span>
                </Button>
                <span className="ml-auto shrink-0 rounded border bg-background px-1.5 py-0.5 font-mono text-[10px] leading-tight tabular-nums text-muted-foreground">
                  {w.layout.w} × {w.layout.h}
                </span>
              </header>
              {widgetSourceError(w.settings) ? (
                <div className="m-3 flex min-h-0 flex-1 flex-col items-center justify-center gap-2 rounded-md border border-dashed bg-muted/15 p-2 text-center text-xs text-muted-foreground">
                  <Icon className="size-5" />
                  {widgetSourceError(w.settings)}
                </div>
              ) : (
                <Slot id={w.id} register={register} editor={w.title} />
              )}
            </div>
          );
        })}
      </GridLayout>
      {visible && valid.length > 0 && (
        <ReadAdmissionContext.Provider value={admit}>
          <ViewOnlyContext.Provider value={true}>
            <EditorTileSlots.Provider value={slots}>
              <OverviewTiles plan={plan} tiles={valid} tags={tags} range={minutes} manual={false} />
            </EditorTileSlots.Provider>
          </ViewOnlyContext.Provider>
        </ReadAdmissionContext.Provider>
      )}
    </div>
  );
}
