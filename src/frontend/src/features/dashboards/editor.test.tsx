import { fireEvent, screen, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { DashboardVisibility } from '@/api/generated/api.types';
import { renderCitadelRoutes } from '@/test/render-citadel';
import { server } from '@/test/server';
import { AppProvider } from '@/lib/context/app-provider';
import Editor from './editor';
import { fromTemplate } from './templates';

it('undoes a new tile, retains pending settings after a conflict, and saves one full definition on retry', async () => {
  const initial = fromTemplate('Blank');
  const requests: unknown[] = [];
  let conflict = true;
  server.use(
    http.get('*/api/v1/tags', () => HttpResponse.json({ tags: [] })),
    http.get('*/api/v1/platforms', () => HttpResponse.json({ platforms: [] })),
    http.get('*/api/v1/profile/preferences', () => HttpResponse.json({})),
    http.get('*/api/v1/application/info', () => HttpResponse.json({})),
    http.get('*/api/v1/alertEvents', () => HttpResponse.json({ pagedResult: { items: [], totalCount: 0 } })),
    http.put('*/api/v1/dashboards/board', async ({ request }) => {
      const body = await request.json();
      requests.push(body);
      return conflict
        ? HttpResponse.json({ status: 409, detail: 'Conflict' }, { status: 409 })
        : HttpResponse.json({ configuration: initial });
    }),
  );
  const close = vi.fn();
  const { user } = renderCitadelRoutes([
    {
      path: '/',
      element: (
        <AppProvider>
          <Editor
            board={{
              id: 'board',
              name: 'My board',
              revision: 3,
              visibility: DashboardVisibility.Private,
              canEdit: true,
              canDelete: true,
            }}
            initial={initial}
            close={close}
          />
        </AppProvider>
      ),
    },
  ]);
  const button = (name: string) => screen.getByRole('button', { name });
  await user.click(button('Add Notes and links'));
  await user.click(button('Undo'));
  expect(button('Save changes')).toBeDisabled();
  await user.click(button('Add Notes and links'));
  const title = screen.getByLabelText('Widget title');
  await user.clear(title);
  await user.type(title, 'Incident contacts');
  const note = '**Page the primary operator**\n\n[Runbook](/platforms)';
  fireEvent.change(screen.getByLabelText('Note'), { target: { value: note } });
  expect(screen.getByText('Page the primary operator', { selector: 'strong' })).toBeVisible();
  expect(screen.getByText('Runbook', { selector: 'a' })).toHaveAttribute('href', '/platforms');
  expect(screen.queryByRole('button', { name: 'Add link' })).toBeNull();
  await user.click(button('Back to widget library'));
  expect(screen.getByRole('heading', { name: 'Widget library' })).toBeVisible();
  await vi.waitFor(() => expect(screen.getByRole('textbox', { name: 'Find widget' })).toHaveFocus());
  await user.click(button('Add Notes and links'));
  expect(screen.getByLabelText('Widget title')).toHaveValue('Notes and links');
  await user.click(button('Remove widget'));
  await user.click(button('Close panel'));
  expect(screen.queryByLabelText('Widget title')).toBeNull();
  await user.click(button('Incident contacts'));
  expect(screen.getByLabelText('Widget title')).toHaveValue('Incident contacts');
  expect(screen.getByLabelText('Note')).toHaveValue(note);
  expect(requests).toHaveLength(0);
  await user.click(button('Save changes'));
  expect(await screen.findByRole('alert')).toHaveTextContent('Your draft is still open');
  expect(screen.getByLabelText('Widget title')).toHaveValue('Incident contacts');
  expect(close).not.toHaveBeenCalled();
  conflict = false;
  await user.click(button('Save changes'));
  await vi.waitFor(() => expect(close).toHaveBeenCalledWith(true));
  expect(requests).toHaveLength(2);
  expect(requests[0]).toEqual(requests[1]);
  expect(requests[1]).toMatchObject({
    revision: 3,
    configuration: { widgets: [{ title: 'Incident contacts', settings: { kind: 'Notes', text: note, links: [] } }] },
  });
});

it('undoes a preset with its displaced neighbor and restores the same duplicate after removal', async () => {
  const initial = fromTemplate('Blank');
  const left = {
    id: 'left',
    title: 'Runbook',
    layout: { x: 0, y: 0, w: 4, h: 3 },
    settings: { kind: 'Notes' as const, text: 'Call the operator', links: [{ label: 'Help', href: '/platforms' }] },
  };
  const right = { ...left, id: 'right', title: 'Contacts', layout: { ...left.layout, x: 4 } };
  initial.widgets = [left, right];
  server.use(
    http.get('*/api/v1/profile/preferences', () => HttpResponse.json({})),
    http.get('*/api/v1/tags', () => HttpResponse.json({ tags: [] })),
  );
  const { user, container } = renderCitadelRoutes([
    {
      path: '/',
      element: (
        <AppProvider>
          <Editor
            board={{
              id: 'board',
              name: 'Operations',
              revision: 1,
              visibility: DashboardVisibility.Private,
              canEdit: true,
              canDelete: true,
            }}
            initial={initial}
            close={vi.fn()}
          />
        </AppProvider>
      ),
    },
  ]);
  const button = (name: string) => screen.getByRole('button', { name });
  await user.click(button('Runbook'));
  await user.click(button('Standard'));
  await user.click(button('Contacts'));
  expect(screen.getByLabelText('Row')).not.toHaveValue(0);
  await user.click(button('Undo'));
  expect(screen.getByLabelText('Row')).toHaveValue(0);
  expect(button('Save changes')).toBeDisabled();
  await user.click(button('Redo'));
  expect(screen.getByLabelText('Row')).not.toHaveValue(0);
  await user.click(button('Duplicate widget'));
  const copyId = container.querySelector('[data-selected=true][id^=tile-]')?.id;
  expect(copyId).toBeTruthy();
  expect(copyId).not.toBe('tile-right');
  await user.click(button('Remove widget'));
  expect(document.getElementById(copyId!)).toBeNull();
  await user.click(button('Undo'));
  expect(document.getElementById(copyId!)).toBeInTheDocument();
  await user.click(button('Redo'));
  expect(document.getElementById(copyId!)).toBeNull();
  await user.click(button('Undo'));
  await user.click(button('Preview'));
  expect(screen.queryByLabelText('Widget title')).toBeNull();
  await user.click(button('Return to editor'));
  expect(document.getElementById(copyId!)).toBeInTheDocument();
});

it('uses the canvas source batch when selecting and restyling a metric', async () => {
  const board = fromTemplate('Infrastructure and capacity');
  board.widgets = board.widgets.filter((w) => w.settings.kind === 'Metrics');
  const ids = board.widgets.map((_, i) => '10000000-0000-4000-8000-00000000000' + (i + 1));
  board.widgets.forEach((w, i) => {
    if (w.settings.kind === 'Metrics') w.settings.source = { ...w.settings.source, id: ids[i] };
  });
  const reads: string[] = [];
  server.use(
    http.get('*/api/v1/dashboards/sources', ({ request }) => {
      reads.push(new URL(request.url).searchParams.get('ids') ?? '');
      return HttpResponse.json(
        ids.map((id, i) => ({ id, name: 'Host ' + (i + 1), status: 'Online', matchesTags: true, platformId: id })),
      );
    }),
    http.get('*/api/v1/platforms/:id/stats', () => HttpResponse.json({ stats: [] })),
    http.get('*/api/v1/profile/preferences', () => HttpResponse.json({})),
    http.get('*/api/v1/platforms', () => HttpResponse.json({ platforms: [] })),
    http.get('*/api/v1/application/info', () => HttpResponse.json({})),
    http.get('*/api/v1/alertEvents', () => HttpResponse.json({ pagedResult: { items: [], totalCount: 0 } })),
    http.get('*/api/v1/tags', () => HttpResponse.json({ tags: [] })),
  );
  const { user } = renderCitadelRoutes([
    {
      path: '/',
      element: (
        <AppProvider>
          <Editor
            initial={board}
            board={{
              id: 'board',
              name: 'Board',
              revision: 1,
              visibility: DashboardVisibility.Private,
              canEdit: true,
              canDelete: true,
            }}
            close={vi.fn()}
          />
        </AppProvider>
      ),
    },
  ]);
  await user.click(screen.getByRole('button', { name: 'Memory usage' }));
  const target = await screen.findByLabelText('Metric resource');
  await vi.waitFor(() => expect(target).toHaveTextContent('Host 2'));
  await user.click(screen.getByRole('radio', { name: 'Green' }));
  await user.click(screen.getByRole('button', { name: 'Use suggested violet' }));
  expect(screen.getByRole('radio', { name: 'Violet' })).toBeChecked();
  await user.click(screen.getByRole('button', { name: 'Undo' }));
  expect(screen.getByRole('radio', { name: 'Green' })).toBeChecked();
  await user.click(screen.getByRole('button', { name: 'CPU usage' }));
  expect(screen.getByLabelText('Metric resource')).toHaveTextContent('Host 1');
  expect(reads).toEqual([ids.join(',')]);
});

it('adds a selected watchlist target to the canvas and saves the pin', async () => {
  const id = '10000000-0000-4000-8000-000000000001';
  let savedPins: unknown;
  const save = vi.fn(async ({ request }: { request: Request }) => {
    const body = await request.json();
    savedPins = body.configuration.widgets[0].settings.pins;
    return HttpResponse.json({ summary: { id: 'board' }, configuration: body.configuration });
  });
  server.use(
    http.get('*/api/v1/dashboards/sources', () =>
      HttpResponse.json([{ id, name: 'Main platform', status: 'Online', matchesTags: true }]),
    ),
    http.get('*/api/v1/tags', () => HttpResponse.json({ tags: [] })),
    http.put('*/api/v1/dashboards/board', save),
  );
  const { user } = renderCitadelRoutes([
    {
      path: '/',
      element: (
        <AppProvider>
          <Editor
            initial={fromTemplate('Blank')}
            board={{
              id: 'board',
              name: 'Board',
              revision: 1,
              visibility: DashboardVisibility.Private,
              canEdit: true,
              canDelete: true,
            }}
            close={vi.fn()}
          />
        </AppProvider>
      ),
    },
  ]);
  await user.click(screen.getByRole('button', { name: 'Add Watchlist' }));
  await user.click(screen.getByRole('combobox', { name: 'Add resource to watchlist' }));
  await user.click(await screen.findByRole('option', { name: 'Main platform' }));
  await vi.waitFor(() =>
    expect(screen.getByRole('combobox', { name: 'Pinned resource 1' })).toHaveTextContent('Main platform'),
  );
  const tile = document.querySelector<HTMLElement>('[data-widget="Watchlist"]')!;
  expect(within(tile).getByText('Main platform')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Save changes' }));
  await vi.waitFor(() => expect(save).toHaveBeenCalledOnce());
  expect(savedPins).toEqual([{ id, kind: 'Platform' }]);
});
