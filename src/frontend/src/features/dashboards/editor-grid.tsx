import { useEffect, useRef, useState } from 'react';
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
import { FrozenPreview } from './preview';
import { ViewOnlyContext } from './tile';
import { GripVertical } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { DashboardWidget } from '@/api/generated/api.types';
import { catalogue, type WidgetKind } from './catalogue';

// RGL handles collisions. Keep gaps in already-valid layouts, including initial mount.
export const dashboardCompactor: typeof verticalCompactor = {
  ...verticalCompactor,
  compact(layout, cols) {
    const overlap = layout.some((item, index) => layout.slice(0, index).some((other) => collides(item, other)));
    return overlap ? verticalCompactor.compact(layout, cols) : cloneLayout(layout);
  },
};

export function nextPosition(widgets: DashboardWidget[], width: number, height: number) {
  for (let y = 0; y <= 512; y++)
    for (let x = 0; x <= 12 - width; x++) {
      if (widgets.every(({ layout: l }) => x + width <= l.x || l.x + l.w <= x || y + height <= l.y || l.y + l.h <= y))
        return { x, y, w: width, h: height };
    }
}
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
  select,
  onMove,
  dragKind,
  setDragKind,
  drop,
}: {
  widgets: DashboardWidget[];
  tags: string[];
  minutes: number;
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
  const layout = widgets.map((w) => ({
    i: w.id,
    ...w.layout,
    minW: w.settings.kind === 'Metrics' ? 4 : 2,
    minH: w.settings.kind === 'Metrics' ? 3 : 2,
    maxH: 24,
  }));
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
      <p className="text-xs text-muted-foreground" aria-live="polite">
        {gesture || (dragKind ? catalogue[dragKind][0] + ' · 6 × 4' : 'Preview paused')}
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
        gridConfig={{ cols: 12, rowHeight: 38, margin: [8, 8], maxRows: 536 }}
        style={{
          minHeight: 240,
          ...({
            '--dashboard-ghost': JSON.stringify(gesture || (dragKind ? catalogue[dragKind][0] + ' · 6 × 4' : '')),
          } as React.CSSProperties),
          ...(gesture || dragKind
            ? { backgroundImage: 'radial-gradient(var(--border) 1px, transparent 1px)', backgroundSize: '12px 12px' }
            : {}),
        }}
        dragConfig={{ handle: '.dashboard-drag-handle' }}
        dropConfig={{
          enabled: true,
          defaultItem: { w: 6, h: 4 },
          onDragOver: (e) => (e.dataTransfer?.types.includes('application/x-citadel-widget') ? { w: 6, h: 4 } : false),
        }}
        droppingItem={{ i: '__drop__', x: 0, y: 0, w: 6, h: 4 }}
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
        {[...widgets]
          .sort((a, b) => a.layout.y - b.layout.y || a.layout.x - b.layout.x)
          .map((w) => (
            <div key={w.id} className="overflow-hidden rounded-md border bg-card p-2">
              <div className="flex items-center gap-1">
                <GripVertical
                  aria-label={'Move ' + w.title}
                  className="dashboard-drag-handle size-4 shrink-0 cursor-move"
                />
                <Button variant="ghost" size="sm" className="truncate" onClick={() => select(w.id)}>
                  {w.title || catalogue[w.settings.kind][0]}
                </Button>
              </div>
              <span className="text-xs text-muted-foreground">
                {w.layout.w} × {w.layout.h}
              </span>
              {!gesture && !dragKind && (
                <ViewOnlyContext.Provider value={true}>
                  <FrozenPreview widget={w} tags={tags} minutes={minutes} />
                </ViewOnlyContext.Provider>
              )}
            </div>
          ))}
      </GridLayout>
    </div>
  );
}
