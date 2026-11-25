import { screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { DashboardVisibility } from '@/api/generated/api.types';
import { renderCitadelRoutes } from '@/test/render-citadel';
import { server } from '@/test/server';
import { AppProvider } from '@/lib/context/app-provider';
import Editor from './editor';
import { fromTemplate } from './templates';

it('cancels a new tile, retains pending settings after a conflict, and saves one full definition on retry', async () => {
  const initial = fromTemplate('Blank');
  const requests: unknown[] = [];
  let conflict = true;
  server.use(
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
  await user.click(button('Cancel settings'));
  expect(button('Save changes')).toBeDisabled();
  await user.click(button('Add Notes and links'));
  const title = screen.getByLabelText('Widget title');
  await user.clear(title);
  await user.type(title, 'Incident contacts');
  await user.type(screen.getByLabelText('Note'), 'Page the primary operator');
  expect(screen.getByText('Page the primary operator', { selector: 'p' })).toBeVisible();
  await user.click(button('Close panel'));
  expect(screen.queryByLabelText('Widget title')).toBeNull();
  await user.click(button('Incident contacts'));
  expect(screen.getByLabelText('Widget title')).toHaveValue('Incident contacts');
  expect(screen.getByLabelText('Note')).toHaveValue('Page the primary operator');
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
    configuration: { widgets: [{ title: 'Incident contacts', settings: { kind: 'Notes' } }] },
  });
});
