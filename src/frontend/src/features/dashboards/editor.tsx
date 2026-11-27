import { useDashboardDraft } from './use-dashboard-draft';
import { EditableDescription } from '@/pages/resource-form-view';
import { DashboardGrid } from './grid';
import { useEffect, useRef, useState, useMemo } from 'react';
import { useNavigate, useBlocker } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import { collides, type Layout } from 'react-grid-layout';
import { ArrowLeft, Check, Copy, GripVertical, Plus, Trash2, X, Undo2, Redo2, Eye } from 'lucide-react';
import {
  DashboardAttentionTab,
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
import { useMutate, useRead } from '@/lib/hooks';
import { requestErrorMessage, requestErrorStatus } from '@/lib/request-error';
import { randomUuid } from '@/lib/utils';
import { categories, catalogue, makeWidget, widgetError, widgetSourceError, type WidgetKind } from './catalogue';
import { EditorGrid, applyGeometry, dashboardCompactor } from './editor-grid';
import { InlineTitleLabel } from '@/components/custom/inline-title-label';
import { CitadelIcons } from '@/lib/icons';
import { nextPosition } from './layout';
import { RangeField, ScopeField } from './display-controls';
import { WidgetSettings } from './widget-settings';
import { dashboardRequirements } from './requirements';
import { WidgetThumbnail } from './widget-thumbnail';

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
  board: DashboardSummaryView;
  initial: DashboardConfiguration;
  close: (saved: boolean) => void;
}) {
  const profile = useRead('getCurrentProfile');
  const admin = profile.data?.data.authorization.isAdministrator && !profile.isError;
  const updatesPreset = useRef(false);
  const [base] = useState(() => ({ board, initial }));
  const draft = useDashboardDraft({ name: board.name, config: initial });
  const { name, config } = draft;
  const [preview, setPreview] = useState(false);
  const [discard, setDiscard] = useState(false);
  const [panel, setPanel] = useState(true);
  const [selected, setSelected] = useState<string>();
  const [originalName, setOriginalName] = useState<string>();
  const titleInput = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (originalName !== undefined) {
      titleInput.current?.focus();
      titleInput.current?.select();
    }
  }, [originalName]);
  const [search, setSearch] = useState('');
  const [dragKind, setDragKind] = useState<WidgetKind>();
  const [error, setError] = useState('');

  const client = useQueryClient();
  const update = useMutate('updateDashboard');
  const create = useMutate('createDashboard');
  const [copyId] = useState(randomUuid);
  const navigate = useNavigate();
  const pending = update.isPending || create.isPending;
  const [savingCopy, setSavingCopy] = useState(false);
  const mutation = savingCopy ? create : update;
  const leaving = useRef(false);
  const dirty = name !== base.board.name || JSON.stringify(config) !== JSON.stringify(base.initial);
  const valid = useMemo(() => config.widgets.filter((w) => !widgetSourceError(w.settings)), [config.widgets]);
  const plan = useMemo(() => dashboardRequirements(valid), [valid]);
  const tile = config.widgets.find((w) => w.id === selected);
  const widgets = (items: DashboardWidget[]) => !pending && draft.change({ config: { ...config, widgets: items } });
  useEffect(() => {
    if (!dirty) return;
    const warn = (e: BeforeUnloadEvent) => {
      e.preventDefault();
      e.returnValue = '';
    };
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  }, [dirty]);
  const save = (after?: () => void, privateCopy = false) => {
    draft.endEdit();
    const issue = !name.trim() ? 'Enter a dashboard name.' : config.widgets.map(widgetError).find(Boolean);
    if (issue) {
      const invalid = config.widgets.find((w) => widgetError(w));
      setPreview(false);
      setPanel(true);
      if (name.trim() && invalid) select(invalid.id);
      else {
        setSelected(undefined);
        setOriginalName(name);
      }
      setError(issue);
      requestAnimationFrame(() =>
        document.querySelector<HTMLElement>(name.trim() ? '#widget-error' : '#dashboard-editor-name')?.focus(),
      );
      return;
    }
    if (pending) return;
    setSavingCopy(privateCopy);
    if (privateCopy) {
      create.mutate(
        {
          data: {
            id: copyId,
            name: name.trim().slice(0, 122) + ' copy',
            configuration: config,
            visibility: DashboardVisibility.Private,
          },
        },
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
      {
        id,
        data: { revision: base.board.revision, name: name.trim(), configuration: config },
      },
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
    draft.endEdit();
    setPanel(true);
    setSelected(id);
    setError('');
    requestAnimationFrame(() =>
      document
        .getElementById('widget-inspector-heading')
        ?.focus({ preventScroll: window.matchMedia('(min-width: 1024px)').matches }),
    );
  };
  const openWidgetLibrary = () => {
    draft.endEdit();
    setPanel(true);
    setSelected(undefined);
    requestAnimationFrame(() =>
      (document.getElementById('dashboard-widget-search') ?? document.getElementById('dashboard-editor-add'))?.focus({
        preventScroll: true,
      }),
    );
  };
  const add = (kind: WidgetKind, from?: DashboardWidget, layout?: Layout, ghostId?: string, updates = false) => {
    if (
      pending ||
      config.widgets.length >= 24 ||
      (kind === 'Metrics' && config.widgets.filter((w) => w.settings.kind === 'Metrics').length >= 8)
    )
      return;
    const item = from ? { ...structuredClone(from), id: randomUuid() } : makeWidget(kind);
    if (updates && item.settings.kind === 'NeedsAttention') {
      item.title = 'Updates';
      item.settings = { ...item.settings, tab: DashboardAttentionTab.Updates };
    }
    const position = layout?.find((p) => p.i === ghostId) ?? nextPosition(config.widgets, item.layout.w, item.layout.h);
    if (!position) {
      setError('No space available at this position.');
      return;
    }
    const { x, y, w, h } = position;
    item.layout = { x, y, w, h };
    select(item.id);
    widgets([...(layout ? applyGeometry(config.widgets, layout) : config.widgets), item]);
    requestAnimationFrame(() => {
      document.getElementById('tile-' + item.id)?.scrollIntoView?.({ block: 'nearest' });
      const field =
        item.settings.kind === 'Watchlist'
          ? '#widget-settings [aria-label="Add resource to watchlist"]'
          : item.settings.kind === 'Metrics'
            ? '#widget-settings [aria-label="Metric resource"]'
            : '#widget-settings input';
      document.querySelector<HTMLElement>(field)?.focus({ preventScroll: true });
    });
  };
  const place = (layout: DashboardWidget['layout'], limit?: number) => {
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
    const items = config.widgets.map((w) =>
      w.id === tile.id && limit !== undefined && 'limit' in w.settings
        ? { ...w, settings: { ...w.settings, limit } }
        : w,
    );
    const moved = { ...layout, i: tile.id, static: true };
    const rest = config.widgets.filter((w) => w.id !== tile.id).map((w) => ({ i: w.id, ...w.layout }));
    if (!rest.some((w) => collides(w, moved))) widgets(items.map((w) => (w.id === tile.id ? { ...w, layout } : w)));
    else {
      const result = dashboardCompactor.compact([moved, ...rest], 12);
      if (result.every((w) => w.y <= 512)) widgets(applyGeometry(items, result));
    }
  };
  const entries = (Object.keys(catalogue) as WidgetKind[])
    .filter((kind) => admin || !['Updates', 'UpcomingSchedules'].includes(kind))
    .map((kind) => ({
      kind,
      label: catalogue[kind].label,
      description: catalogue[kind].description,
      updates: false,
    }));
  entries.push({
    kind: 'NeedsAttention',
    label: 'Updates',
    description: 'Resources with a recorded image or Git update.',
    updates: true,
  });
  const panelOpen = panel && !preview;
  const metricCount = config.widgets.filter((w) => w.settings.kind === 'Metrics').length;
  return (
    <div
      className="dashboard-builder"
      onBlurCapture={draft.endEdit}
      onKeyDown={(e) => {
        if (pending || e.defaultPrevented || e.nativeEvent.isComposing) return;
        if (e.key === 'Enter') draft.endEdit();
        if (
          !preview &&
          (e.ctrlKey || e.metaKey) &&
          e.key.toLowerCase() === 'z' &&
          !(e.target as HTMLElement).closest('input,textarea,[contenteditable=true]')
        ) {
          e.preventDefault();
          if (e.shiftKey) draft.redo();
          else draft.undo();
        } else if (e.key === 'Escape') {
          if (preview) setPreview(false);
          else if (selected) openWidgetLibrary();
          else setPanel(false);
        }
      }}>
      <fieldset disabled={pending} className="min-w-0">
        <div data-panel={panelOpen} className="dashboard-builder-layout">
          <header className="dashboard-builder-toolbar">
            <div className="flex min-w-0 flex-wrap items-center gap-3">
              <CitadelIcons.Dashboard
                aria-hidden="true"
                className="box-content size-5 shrink-0 rounded-lg border border-primary/15 bg-primary/5 p-2.5 text-primary"
              />
              <div className="min-w-48 flex-1">
                {originalName === undefined || preview ? (
                  <InlineTitleLabel
                    value={name || 'Dashboard'}
                    onEdit={preview ? undefined : () => setOriginalName(name)}
                    status={<Badge variant="outline">{preview ? 'Preview' : 'Editing'}</Badge>}
                  />
                ) : (
                  <div className="flex w-full items-center gap-2">
                    <Input
                      ref={titleInput}
                      id="dashboard-editor-name"
                      aria-label="Dashboard name"
                      value={name}
                      maxLength={128}
                      className="w-full font-semibold"
                      onChange={(e) => draft.change({ name: e.target.value })}
                      onKeyDown={(e) => {
                        if (e.nativeEvent.isComposing || (e.key !== 'Escape' && e.key !== 'Enter')) return;
                        e.preventDefault();
                        if (e.key === 'Escape') draft.change({ name: originalName });
                        setOriginalName(undefined);
                        draft.endEdit();
                        requestAnimationFrame(() =>
                          document
                            .querySelector<HTMLButtonElement>('.dashboard-builder-toolbar [aria-label="Edit name"]')
                            ?.focus(),
                        );
                      }}
                    />
                    <Button
                      type="button"
                      variant="outline"
                      size="icon-sm"
                      aria-label="Apply name"
                      onClick={() => {
                        setOriginalName(undefined);
                        draft.endEdit();
                      }}>
                      <Check />
                    </Button>
                  </div>
                )}
                <EditableDescription
                  maxLength={4096}
                  value={config.description ?? ''}
                  readOnly={preview}
                  onSave={(description) =>
                    draft.change({ config: { ...config, description: description.trim() || undefined } })
                  }
                />
              </div>
              <div className="ms-auto flex flex-wrap items-center gap-2">
                <Button variant="outline" disabled={!draft.canUndo || preview} onClick={draft.undo} aria-label="Undo">
                  <Undo2 />
                </Button>
                <Button variant="outline" disabled={!draft.canRedo || preview} onClick={draft.redo} aria-label="Redo">
                  <Redo2 />
                </Button>
                <Button
                  variant="outline"
                  onClick={() => {
                    draft.endEdit();
                    setOriginalName(undefined);
                    setPreview(!preview);
                  }}>
                  <Eye />
                  {preview ? 'Return to editor' : 'Preview'}
                </Button>
                <Button variant="outline" onClick={() => (dirty ? setDiscard(true) : close(false))}>
                  Discard
                </Button>
                <Button disabled={!dirty || pending} onClick={() => save()}>
                  <Check /> Save changes
                </Button>
              </div>
            </div>
            {requestErrorStatus(update.error) === 409 && (
              <Button disabled={pending} variant="outline" onClick={() => save(undefined, true)}>
                Save as private copy
              </Button>
            )}
            {(error || mutation.isError) && (
              <p role="alert" className="text-destructive">
                {error ||
                  (requestErrorStatus(mutation.error) === 409
                    ? 'Another editor changed this dashboard. Your draft is still open.'
                    : requestErrorMessage(mutation.error))}
              </p>
            )}
            {!preview && (
              <div className="dashboard-edit-tools mt-3 flex flex-wrap items-center gap-2 border-t pt-3">
                <RangeField
                  value={config.rangeMinutes}
                  change={(rangeMinutes) => draft.change({ config: { ...config, rangeMinutes } })}
                />
                <ScopeField value={config.tags} change={(tags) => draft.change({ config: { ...config, tags } })} />
                <div className="ms-auto flex items-center gap-2">
                  {dirty && (
                    <Badge variant="outline" className="border-warning/30 bg-warning/10 text-warning">
                      Unsaved changes
                    </Badge>
                  )}
                  <span className="text-xs tabular-nums text-muted-foreground">
                    {config.widgets.length} / 24 widgets
                  </span>
                </div>
                <Button
                  id="dashboard-editor-add"
                  variant="outline"
                  size="sm"
                  aria-expanded={panelOpen && !tile}
                  aria-controls={panelOpen ? 'widget-settings' : undefined}
                  onClick={openWidgetLibrary}>
                  <Plus />
                  Add widget
                </Button>
              </div>
            )}
          </header>
          <div className="dashboard-builder-canvas" data-editing={!preview}>
            {preview ? (
              <DashboardGrid configuration={config} />
            ) : (
              <EditorGrid
                valid={valid}
                plan={plan}
                widgets={config.widgets}
                selected={selected}
                tags={config.tags}
                minutes={config.rangeMinutes}
                select={select}
                onMove={(layout) => widgets(applyGeometry(config.widgets, layout))}
                dragKind={dragKind}
                setDragKind={setDragKind}
                drop={(kind, layout, item) => add(kind, undefined, layout, item.i, updatesPreset.current)}
              />
            )}
          </div>
          {panelOpen && (
            <aside id="widget-settings" className="dashboard-builder-panel">
              <div className="dashboard-inspector-bar flex shrink-0 items-center justify-between gap-2 border-b px-4 py-3">
                <div className="flex min-w-0 items-center gap-2">
                  {tile && (
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon-sm"
                      aria-label="Back to widget library"
                      title="Back to widget library"
                      onClick={openWidgetLibrary}>
                      <ArrowLeft aria-hidden="true" />
                    </Button>
                  )}
                  <h2 tabIndex={-1} id="widget-inspector-heading" className="text-sm font-semibold focus:outline-none">
                    {tile ? 'Widget settings' : 'Widget library'}
                  </h2>
                </div>
                {tile && (
                  <div className="ml-auto flex items-center gap-1">
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
                        openWidgetLibrary();
                      }}>
                      <Trash2 />
                    </Button>
                  </div>
                )}
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="Close panel"
                  onClick={() => {
                    setPanel(false);
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
                  <div key={tile.id} className="dashboard-inspector-fields min-h-0 flex-1 overflow-y-auto p-4">
                    <WidgetSettings
                      batches={plan.batches}
                      tags={config.tags}
                      widget={tile}
                      change={(item) => widgets(config.widgets.map((w) => (w.id === item.id ? item : w)))}
                      place={place}
                    />
                  </div>
                  {error && (
                    <p tabIndex={-1} id="widget-error" className="px-5 pb-3 text-destructive">
                      {error}
                    </p>
                  )}
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
                  {!entries.some(({ label, description }) =>
                    (label + ' ' + description).toLowerCase().includes(search.toLowerCase()),
                  ) && <p>No matching widgets.</p>}
                  {categories.map((category) => {
                    const matching = entries.filter(
                      ({ kind, label, description }) =>
                        catalogue[kind].category === category &&
                        (label + ' ' + description).toLowerCase().includes(search.toLowerCase()),
                    );
                    return (
                      matching.length > 0 && (
                        <section key={category} className="space-y-2">
                          <h3 className="text-xs font-medium text-muted-foreground">{category}</h3>
                          {matching.map(({ kind, label, description, updates }) => {
                            const { size } = catalogue[kind];
                            const limit =
                              config.widgets.length >= 24
                                ? 'A dashboard can contain up to 24 widgets.'
                                : kind === 'Metrics' && metricCount >= 8
                                  ? 'A dashboard can contain up to eight metric charts.'
                                  : undefined;
                            const disabled = pending || !!limit;
                            return (
                              <div
                                key={label}
                                draggable={!disabled}
                                onDragStart={(e) => {
                                  e.dataTransfer.setData('text/plain', label);
                                  e.dataTransfer.setData('application/x-citadel-widget', kind);
                                  updatesPreset.current = updates;
                                  setDragKind(kind);
                                }}
                                onDragEnd={() => setDragKind(undefined)}
                                className="dashboard-library-item group flex items-center gap-2 rounded-md border p-2">
                                <WidgetThumbnail kind={kind} />
                                <div className="min-w-0 grow">
                                  <div className="text-sm font-medium">
                                    {label}{' '}
                                    <span className="text-xs font-normal text-muted-foreground">
                                      {size.w} × {size.h}
                                    </span>
                                  </div>
                                  <p className="text-xs text-muted-foreground">{limit ?? description}</p>
                                </div>
                                <Button
                                  variant="outline"
                                  size="icon-sm"
                                  aria-label={'Add ' + label}
                                  disabled={disabled}
                                  onClick={() => add(kind, undefined, undefined, undefined, updates)}>
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
      <Dialog open={discard} onOpenChange={setDiscard}>
        <DialogContent>
          <DialogTitle>Discard dashboard changes?</DialogTitle>
          <DialogDescription>Your saved dashboard will keep its previous configuration.</DialogDescription>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDiscard(false)}>
              Keep editing
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                leaving.current = true;
                close(false);
              }}>
              Discard changes
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <LeaveGuard released={() => leaving.current} dirty={dirty} pending={pending} save={save} />
    </div>
  );
}
