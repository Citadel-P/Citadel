import { PlatformView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useLocation } from 'react-router';
import { PlatformInfoActions } from './actions';

const platform = { id: 'platform-1', name: 'edge-docker' } as PlatformView;
const CurrentLocation = () => {
  const location = useLocation();
  return <span data-testid="location">{location.pathname + location.search + location.hash}</span>;
};

describe('Platform deletion navigation', () => {
  it.each([
    ['/platforms/edit/platform-1', '/platforms'],
    ['/platforms/edit/platform-1/#config', '/platforms'],
    ['/platforms?search=docker', '/platforms?search=docker'],
    ['/platforms/edit/another-platform', '/platforms/edit/another-platform'],
  ])('returns to the list only when deleting the edited platform at %s', async (route, expected) => {
    const deleted = vi.fn();
    server.use(
      http.delete('http://localhost/api/v1/platforms', async ({ request }) => {
        deleted(await request.json());
        return new HttpResponse(null, { status: 204 });
      }),
    );
    const { user } = renderCitadel(
      <>
        <PlatformInfoActions.delete resource={platform} />
        <CurrentLocation />
      </>,
      { route },
    );

    await user.click(screen.getByRole('button', { name: 'Delete' }));
    const dialog = within(screen.getByRole('dialog'));
    await user.type(dialog.getByRole('textbox'), platform.name);
    await user.click(dialog.getByRole('button', { name: 'Delete' }));

    await waitFor(() => expect(deleted).toHaveBeenCalledWith({ ids: [platform.id] }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(screen.getByTestId('location').textContent).toBe(expected);
  });

  it('stays on the edit page when deletion fails', async () => {
    const errorLog = vi.spyOn(console, 'error').mockImplementation(() => {});
    server.use(
      http.delete('http://localhost/api/v1/platforms', () =>
        HttpResponse.json({ title: 'Deletion failed', status: 500 }, { status: 500 }),
      ),
    );
    try {
      const route = '/platforms/edit/platform-1';
      const { user } = renderCitadel(
        <>
          <PlatformInfoActions.delete resource={platform} />
          <CurrentLocation />
        </>,
        { route },
      );
      await user.click(screen.getByRole('button', { name: 'Delete' }));
      const dialog = within(screen.getByRole('dialog'));
      await user.type(dialog.getByRole('textbox'), platform.name);
      await user.click(dialog.getByRole('button', { name: 'Delete' }));

      await waitFor(() => expect(errorLog).toHaveBeenCalled());
      await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
      expect(screen.getByTestId('location')).toHaveTextContent(route);
    } finally {
      errorLog.mockRestore();
    }
  });
});
