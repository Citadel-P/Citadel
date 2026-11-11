import { screen, within, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/server';
import { renderCitadel } from '@/test/render-citadel';
import { InstanceRestartButton } from './restart';

it('confirms restart, sends the current boot ID and prevents duplicate clicks', async () => {
  const user = userEvent.setup();
  const called = vi.fn();
  server.use(
    http.get('http://localhost/api/v1/instance/restart', () =>
      HttpResponse.json({ available: true, pending: false, bootId: 'boot-id' }),
    ),
    http.post('http://localhost/api/v1/instance/restart', async ({ request }) => {
      called(await request.json());
      return new HttpResponse(null, { status: 202 });
    }),
  );
  renderCitadel(<InstanceRestartButton />);
  const button = screen.getByRole('button', { name: 'Restart Core' });
  await waitFor(() => expect(button).toBeEnabled());
  await user.click(button);
  expect(called).not.toHaveBeenCalled();
  const dialog = screen.getByRole('dialog');
  expect(dialog).toHaveTextContent('Only saved settings will be applied.');
  await user.click(within(dialog).getByRole('button', { name: 'Restart Core' }));
  await waitFor(() => expect(called).toHaveBeenCalledWith({ bootId: 'boot-id' }));
  expect(await screen.findByRole('button', { name: 'Restarting…' })).toBeDisabled();
});

it('disables restart when the host does not support it', async () => {
  server.use(
    http.get('http://localhost/api/v1/instance/restart', () =>
      HttpResponse.json({ available: false, pending: false, bootId: null }),
    ),
  );
  renderCitadel(<InstanceRestartButton />);
  expect(screen.getByRole('button', { name: 'Restart Core' })).toBeDisabled();
});
