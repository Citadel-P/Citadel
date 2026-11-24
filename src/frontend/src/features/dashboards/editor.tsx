import { useEffect, useRef, useState } from 'react';
import { useNavigate, useBlocker } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import { collides, type Layout } from 'react-grid-layout';
import { Copy, Layers, Plus, Trash2 } from 'lucide-react';
import {
  DashboardVisibility,
  type DashboardConfiguration,
  type DashboardWidget,
  type DashboardSummaryView,
} from '@/api/generated/api.types';
import { resourceQueryKey } from '@/api/dispatch';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Dialog, DialogContent, DialogTitle, DialogFooter, DialogDescription } from '@/components/ui/dialog';
import { useMutate } from '@/lib/hooks';
import { requestErrorMessage, requestErrorStatus } from '@/lib/request-error';
import { randomUuid } from '@/lib/utils';
import { catalogue, makeWidget, widgetError, type WidgetKind } from './catalogue';
import { EditorGrid, nextPosition, applyGeometry, dashboardCompactor } from './editor-grid';
import { WidgetSettings, Field, ScopeField, RangeField } from './widget-settings';

function LeaveGuard({
  dirty,
  released,
  save,
  pending,
}: {
  dirty: boolean;
  released: () => boolean;
  save: (done: () => void) => void;
  pending: boolean;
}) {
  const blocker = useBlocker(() => !released() && (dirty || pending));
  if (blocker.state !== 'blocked') return null;
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) blocker.reset();
      }}>
      <DialogContent>
        <DialogTitle>Leave dashboard editor?</DialogTitle>
        <DialogDescription>Your dashboard has unsaved changes.</DialogDescription>
        <DialogFooter>
          <Button variant="outline" disabled={pending} onClick={() => blocker.reset()}>
            Keep editing
          </Button>
          <Button variant="outline" disabled={pending} onClick={() => blocker.proceed()}>
            Discard
          </Button>
          <Button disabled={pending} onClick={() => save(() => blocker.proceed())}>
            Save and leave
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export default function Editor({
  board,
  initial,
  close,
}: {
  board?: DashboardSummaryView;
  initial: DashboardConfiguration;
  close: (saved: boolean) => void;
}) {
  const [base] = useState(() => ({ board, initial }));
  const [name, setName] = useState(board?.name ?? 'My Overview');
  const [config, setConfig] = useState(initial);
  const [selected, setSelected] = useState<string>();
  const [search, setSearch] = useState('');
  const [dragKind, setDragKind] = useState<WidgetKind>();
  const [error, setError] = useState('');
  const snapshot = useRef<DashboardWidget[]>([]);
  const client = useQueryClient();
  const update = useMutate('updateDashboard');
  const create = useMutate('createDashboard');
  const [copyId] = useState(randomUuid);
  const navigate = useNavigate();
  const mutation = board ? update : create;
  const leaving = useRef(false);
  const dirty = name !== (base.board?.name ?? 'My Overview') || JSON.stringify(config) !== JSON.stringify(base.initial);
  const tile = config.widgets.find((w) => w.id === selected);
  const widgets = (items: DashboardWidget[]) => setConfig((c) => ({ ...c, widgets: items }));
  useEffect(() => {
    if (!dirty) return;
    const warn = (e: BeforeUnloadEvent) => {
      e.preventDefault();
      e.returnValue = '';
    };
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  }, [dirty]);
  const save = (after?: () => void) => {
    const issue = !name.trim() ? 'Enter a dashboard name.' : config.widgets.map(widgetError).find(Boolean);
    if (issue) {
      setError(issue);
      return;
    }
    if (mutation.isPending) return;
    if (!base.board) {
      create.mutate(
        { data: { id: copyId, name: name.trim(), configuration: config, visibility: DashboardVisibility.Private } },
        {
          onSuccess: ({ data }) => {
            leaving.current = true;
            void client.invalidateQueries({ queryKey: ['listDashboards'] });
            close(true);
            if (after) after();
            else navigate('/dashboards/' + data.summary.id);
          },
        },
      );
      return;
    }
    const id = base.board.id;
    update.mutate(
      { id: base.board.id, data: { revision: base.board.revision, name: name.trim(), configuration: config } },
      {
        onError: (err) => {
          if (requestErrorStatus(err) === 409)
            void client.invalidateQueries({ queryKey: resourceQueryKey('getDashboard', { id }), exact: true });
        },
        onSuccess: (result) => {
          client.setQueryData(resourceQueryKey('getDashboard', { id }), result);
          void client.invalidateQueries({ queryKey: ['listDashboards'] });
          leaving.current = true;
          after?.();
          close(true);
        },
      },
    );
  };
  const select = (id: string) => {
    snapshot.current = config.widgets;
    setSelected(id);
    setError('');
  };
  const catalogueFocus = () => {
    setSelected(undefined);
    requestAnimationFrame(() => document.getElementById('dashboard-widget-search')?.focus());
  };
  const add = (kind: WidgetKind, from?: DashboardWidget, layout?: Layout, ghostId?: string) => {
    if (
      config.widgets.length >= 24 ||
      (kind === 'Metrics' && config.widgets.filter((w) => w.settings.kind === 'Metrics').length >= 8)
    )
      return;
    const item = from ? { ...structuredClone(from), id: randomUuid() } : makeWidget(kind);
    const position = layout?.find((p) => p.i === ghostId) ?? nextPosition(config.widgets, item.layout.w, item.layout.h);
    if (!position) {
      setError('No space available at this position.');
      return;
    }
    const { x, y, w, h } = position;
    item.layout = { x, y, w, h };
    select(item.id);
    widgets([...(layout ? applyGeometry(config.widgets, layout) : config.widgets), item]);
    requestAnimationFrame(() => document.querySelector<HTMLInputElement>('#widget-settings input')?.focus());
  };
  const place = (layout: DashboardWidget['layout']) => {
    if (
      !tile ||
      !Object.values(layout).every(Number.isInteger) ||
      layout.x < 0 ||
      layout.y < 0 ||
      layout.y > 512 ||
      layout.w < (tile.settings.kind === 'Metrics' ? 4 : 2) ||
      layout.h < (tile.settings.kind === 'Metrics' ? 3 : 2) ||
      layout.x + layout.w > 12 ||
      layout.h > 24
    )
      return;
    const moved = { ...layout, i: tile.id, static: true };
    const rest = config.widgets.filter((w) => w.id !== tile.id).map((w) => ({ i: w.id, ...w.layout }));
    if (!rest.some((w) => collides(w, moved)))
      widgets(config.widgets.map((w) => (w.id === tile.id ? { ...w, layout } : w)));
    else {
      const result = dashboardCompactor.compact([moved, ...rest], 12);
      if (result.every((w) => w.y <= 512)) widgets(applyGeometry(config.widgets, result));
    }
  };
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-3">
        <h1 className="text-2xl font-semibold">Edit dashboard</h1>
        {dirty && <span className="text-sm text-muted-foreground">Unsaved changes</span>}
        <div className="ml-auto flex gap-2">
          <Button variant="outline" disabled={mutation.isPending} onClick={() => close(false)}>
            Cancel
          </Button>
          <Button disabled={(!!board && !dirty) || mutation.isPending} onClick={() => save()}>
            Save changes
          </Button>
        </div>
      </div>
      {(error || mutation.isError) && (
        <p role="alert" className="text-destructive">
          {error ||
            (requestErrorStatus(mutation.error) === 409
              ? 'Another editor changed this dashboard. Your draft is still open.'
              : requestErrorMessage(mutation.error))}
        </p>
      )}
      <fieldset disabled={mutation.isPending} className="space-y-4">
        <div className="flex flex-wrap items-end gap-4">
          <Field label="Dashboard name">
            <Input value={name} maxLength={128} onChange={(e) => setName(e.target.value)} />
          </Field>
          <RangeField value={config.rangeMinutes} change={(rangeMinutes) => setConfig({ ...config, rangeMinutes })} />
          <ScopeField value={config.tags} change={(tags) => setConfig({ ...config, tags })} />
        </div>
        <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_20rem]">
          <EditorGrid
            widgets={config.widgets}
            tags={config.tags}
            minutes={config.rangeMinutes}
            select={select}
            onMove={(layout) => widgets(applyGeometry(config.widgets, layout))}
            dragKind={dragKind}
            setDragKind={setDragKind}
            drop={(kind, layout, item) => add(kind, undefined, layout, item.i)}
          />
          <aside
            id="widget-settings"
            className="space-y-3 rounded-md border bg-card p-4 lg:sticky lg:top-4 lg:max-h-[calc(100dvh-6rem)] lg:self-start lg:overflow-y-auto">
            {tile ? (
              <>
                <h2 className="font-medium">Widget settings</h2>
                <WidgetSettings
                  widget={tile}
                  change={(item) => widgets(config.widgets.map((w) => (w.id === item.id ? item : w)))}
                  place={place}
                />
                <div className="flex flex-wrap gap-2">
                  <Button
                    size="sm"
                    onClick={() => {
                      const issue = widgetError(tile);
                      setError(issue ?? '');
                      if (!issue) catalogueFocus();
                    }}>
                    Apply
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => {
                      widgets(snapshot.current);
                      catalogueFocus();
                      setError('');
                    }}>
                    Cancel settings
                  </Button>
                  <Button
                    variant="outline"
                    size="icon-sm"
                    aria-label="Duplicate widget"
                    disabled={
                      config.widgets.length >= 24 ||
                      (tile.settings.kind === 'Metrics' &&
                        config.widgets.filter((w) => w.settings.kind === 'Metrics').length >= 8)
                    }
                    onClick={() => add(tile.settings.kind, tile)}>
                    <Copy />
                  </Button>
                  <Button
                    variant="outline"
                    size="icon-sm"
                    aria-label="Remove widget"
                    onClick={() => {
                      widgets(config.widgets.filter((w) => w.id !== tile.id));
                      catalogueFocus();
                    }}>
                    <Trash2 />
                  </Button>
                </div>
              </>
            ) : (
              <>
                <h2 className="font-medium">Add widget</h2>
                <Input
                  id="dashboard-widget-search"
                  aria-label="Find widget"
                  placeholder="Find widget…"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                />
                {(Object.keys(catalogue) as WidgetKind[])
                  .filter((k) => catalogue[k][0].toLowerCase().includes(search.toLowerCase()))
                  .map((kind) => {
                    const [label, description] = catalogue[kind];
                    const disabled =
                      config.widgets.length >= 24 ||
                      (kind === 'Metrics' && config.widgets.filter((w) => w.settings.kind === 'Metrics').length >= 8);
                    return (
                      <div
                        key={kind}
                        draggable={!disabled}
                        onDragStart={(e) => {
                          e.dataTransfer.setData('text/plain', label);
                          e.dataTransfer.setData('application/x-citadel-widget', kind);
                          setDragKind(kind);
                        }}
                        onDragEnd={() => setDragKind(undefined)}
                        className="flex items-start gap-2 border-t pt-2">
                        <Layers className="mt-1 size-4 shrink-0 text-muted-foreground" />
                        <div className="grow">
                          <div className="text-sm">{label}</div>
                          <p className="text-xs text-muted-foreground">{description}</p>
                        </div>
                        <Button
                          variant="outline"
                          size="icon-sm"
                          aria-label={'Add ' + label}
                          disabled={disabled}
                          onClick={() => add(kind)}>
                          <Plus />
                        </Button>
                      </div>
                    );
                  })}
              </>
            )}
          </aside>
        </div>
      </fieldset>
      <LeaveGuard released={() => leaving.current} dirty={dirty} pending={mutation.isPending} save={save} />
    </div>
  );
}
