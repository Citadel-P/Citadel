import { useEffect, useRef, useState } from 'react';
import { useNavigate, useBlocker } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import { collides, type Layout } from 'react-grid-layout';
import { Check, Copy, GripVertical, Plus, Trash2, X } from 'lucide-react';
import {
  DashboardVisibility,
  type DashboardConfiguration,
  type DashboardWidget,
  type DashboardSummaryView,
} from '@/api/generated/api.types';
import { resourceQueryKey } from '@/api/dispatch';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Dialog, DialogContent, DialogTitle, DialogFooter, DialogDescription } from '@/components/ui/dialog';
import { useMutate } from '@/lib/hooks';
import { requestErrorMessage, requestErrorStatus } from '@/lib/request-error';
import { randomUuid } from '@/lib/utils';
import { categories, catalogue, makeWidget, widgetError, type WidgetKind } from './catalogue';
import { EditorGrid, applyGeometry, dashboardCompactor } from './editor-grid';
import { PageHeader } from '@/components/custom/page-header';
import { CitadelIcons } from '@/lib/icons';
import { nextPosition } from './layout';
import { RangeField } from './display-controls';
import { WidgetSettings, ScopeField } from './widget-settings';

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
  const [libraryOpen, setLibraryOpen] = useState(initial.widgets.length === 0);
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
    if (selected === id) return;
    snapshot.current = config.widgets;
    setSelected(id);
    setError('');
    requestAnimationFrame(() =>
      document
        .getElementById('widget-inspector-heading')
        ?.focus({ preventScroll: window.matchMedia('(min-width: 1024px)').matches }),
    );
  };
  const catalogueFocus = () => {
    setSelected(undefined);
    requestAnimationFrame(() =>
      (document.getElementById('dashboard-widget-search') ?? document.getElementById('dashboard-editor-add'))?.focus({
        preventScroll: true,
      }),
    );
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
      layout.w < catalogue[tile.settings.kind].minW ||
      layout.h < catalogue[tile.settings.kind].minH ||
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
  const panelOpen = !!tile || libraryOpen;
  return (
    <div className="dashboard-builder">
      <fieldset disabled={mutation.isPending} className="min-w-0">
        <div data-panel={panelOpen} className="dashboard-builder-layout">
          <header className="dashboard-builder-toolbar">
            <PageHeader
              title={
                <span className="flex flex-wrap items-center gap-2">
                  Edit dashboard
                  {dirty && (
                    <Badge
                      variant="outline"
                      className="gap-1.5 border-amber-500/30 bg-amber-500/10 font-normal text-amber-700 dark:text-amber-300">
                      <span className="size-1.5 rounded-full bg-amber-500" />
                      Unsaved changes
                    </Badge>
                  )}
                </span>
              }
              icon={<CitadelIcons.Dashboard className="size-5" />}
              description={name}
              actions={
                <>
                  <Button variant="outline" disabled={mutation.isPending} onClick={() => close(false)}>
                    Cancel
                  </Button>
                  <Button disabled={(!!board && !dirty) || mutation.isPending} onClick={() => save()}>
                    <Check /> Save changes
                  </Button>
                </>
              }
            />
            {(error || mutation.isError) && (
              <p role="alert" className="text-destructive">
                {error ||
                  (requestErrorStatus(mutation.error) === 409
                    ? 'Another editor changed this dashboard. Your draft is still open.'
                    : requestErrorMessage(mutation.error))}
              </p>
            )}
            <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-2 border-t pt-3">
              <label htmlFor="dashboard-editor-name" className="flex items-center gap-2 text-sm">
                Dashboard name
                <Input
                  id="dashboard-editor-name"
                  className="w-40"
                  value={name}
                  maxLength={128}
                  onChange={(e) => setName(e.target.value)}
                />
              </label>
              <RangeField
                value={config.rangeMinutes}
                change={(rangeMinutes) => setConfig({ ...config, rangeMinutes })}
              />
              <ScopeField value={config.tags} change={(tags) => setConfig({ ...config, tags })} />
              <Button
                id="dashboard-editor-add"
                className="ms-auto"
                variant="outline"
                size="sm"
                aria-expanded={libraryOpen && !tile}
                aria-controls={panelOpen ? 'widget-settings' : undefined}
                onClick={() => {
                  setLibraryOpen(true);
                  setSelected(undefined);
                  requestAnimationFrame(() => document.getElementById('dashboard-widget-search')?.focus());
                }}>
                <Plus />
                Add widget
              </Button>
            </div>
          </header>
          <div className="dashboard-builder-canvas">
            <EditorGrid
              widgets={config.widgets}
              selected={selected}
              tags={config.tags}
              minutes={config.rangeMinutes}
              select={select}
              onMove={(layout) => widgets(applyGeometry(config.widgets, layout))}
              dragKind={dragKind}
              setDragKind={setDragKind}
              drop={(kind, layout, item) => add(kind, undefined, layout, item.i)}
            />
          </div>
          {panelOpen && (
            <aside id="widget-settings" className="dashboard-builder-panel">
              <div className="flex shrink-0 items-center justify-between gap-2 border-b bg-muted/20 px-5 py-4">
                <h2 tabIndex={-1} id="widget-inspector-heading" className="text-sm font-semibold focus:outline-none">
                  {tile ? 'Widget settings' : 'Widget library'}
                </h2>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="Close panel"
                  onClick={() => {
                    setLibraryOpen(false);
                    setSelected(undefined);
                    requestAnimationFrame(() =>
                      document.getElementById('dashboard-editor-add')?.focus({ preventScroll: true }),
                    );
                  }}>
                  <X />
                </Button>
              </div>
              {tile ? (
                <>
                  <div className="min-h-0 flex-1 overflow-y-auto p-5">
                    <WidgetSettings
                      widget={tile}
                      change={(item) => widgets(config.widgets.map((w) => (w.id === item.id ? item : w)))}
                      place={place}
                    />
                  </div>
                  <div className="flex shrink-0 flex-wrap items-center gap-2 border-t px-4 py-3">
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
                      variant="ghost"
                      size="icon-sm"
                      className="ml-auto"
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
                      variant="ghost"
                      size="icon-sm"
                      className="text-destructive"
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
                <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-5">
                  <Input
                    id="dashboard-widget-search"
                    aria-label="Find widget"
                    placeholder="Find widget…"
                    value={search}
                    onChange={(e) => setSearch(e.target.value)}
                  />
                  {categories.map((category) => {
                    const matching = (Object.keys(catalogue) as WidgetKind[]).filter(
                      (k) =>
                        catalogue[k].category === category &&
                        (catalogue[k].label + ' ' + catalogue[k].description)
                          .toLowerCase()
                          .includes(search.toLowerCase()),
                    );
                    return (
                      matching.length > 0 && (
                        <section key={category} className="space-y-1">
                          <h3 className="text-xs font-medium text-muted-foreground">{category}</h3>
                          {matching.map((kind) => {
                            const { label, description, Icon } = catalogue[kind];
                            const disabled =
                              config.widgets.length >= 24 ||
                              (kind === 'Metrics' &&
                                config.widgets.filter((w) => w.settings.kind === 'Metrics').length >= 8);
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
                                className="group flex items-start gap-2 rounded-md p-2 hover:bg-muted/50">
                                <Icon className="mt-1 size-4 shrink-0 text-muted-foreground" />
                                <div className="min-w-0 grow">
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
                                <GripVertical
                                  aria-hidden="true"
                                  className="mt-2 size-3 shrink-0 cursor-grab text-muted-foreground"
                                />
                              </div>
                            );
                          })}
                        </section>
                      )
                    );
                  })}
                </div>
              )}
            </aside>
          )}
        </div>
      </fieldset>
      <LeaveGuard released={() => leaving.current} dirty={dirty} pending={mutation.isPending} save={save} />
    </div>
  );
}
