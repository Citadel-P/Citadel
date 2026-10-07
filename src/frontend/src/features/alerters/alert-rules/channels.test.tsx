import { screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { AlertDestination, type AlertChannelInput, type AlertChannelView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { AlertRuleComponents } from './index';

it('tests an Email channel without saving, then persists it with SMTP settings', async () => {
  const capabilities = { canRead: true, canWrite: true, canExecute: true };
  const channels: AlertChannelView[] = [];
  const verified: unknown[] = [];
  const saved: unknown[] = [];
  server.use(
    http.get('http://localhost/api/v1/alertRules/channels', () => HttpResponse.json({ channels, capabilities })),
    http.post('http://localhost/api/v1/alertRules/channels/verify', async ({ request }) => {
      verified.push(await request.json());
      return new HttpResponse(null, { status: 204 });
    }),
    http.post('http://localhost/api/v1/alertRules/channels', async ({ request }) => {
      const input = (await request.json()) as AlertChannelInput;
      saved.push(input);
      channels.push({
        ...input,
        name: input.name ?? '',
        id: '00000000-0000-0000-0000-000000000001',
        createdAt: '2026-01-01T00:00:00Z',
        createdByActorId: '00000000-0000-0000-0000-000000000002',
        capabilities,
      });
      return HttpResponse.json({ id: channels[0].id });
    }),
  );
  const Content = AlertRuleComponents.Content!;
  const { user } = renderCitadel(<Content items={[]} actions={{}} isLoading={false} />);
  const add = await screen.findByRole('button', { name: 'Add Channel' });
  await waitFor(() => expect(add).toBeEnabled());
  await user.click(add);
  const dialog = within(screen.getByRole('dialog'));
  await user.type(dialog.getByPlaceholderText('e.g. Engineering Team Channel'), 'Operations email');
  await user.click(dialog.getByRole('combobox'));
  await user.click(screen.getByRole('option', { name: 'Email' }));
  expect(dialog.getByPlaceholderText('smtp://...')).toBeVisible();
  const url =
    'smtp://demo:password@smtp.example.com:587/?from=alerts@example.com&to=ops@example.com&requirestarttls=yes';
  await user.click(dialog.getByLabelText('Channel URL'));
  await user.paste(url);
  await user.click(dialog.getByRole('button', { name: 'Send Test Notification' }));
  await waitFor(() =>
    expect(verified).toEqual([{ name: 'Operations email', alertDestination: AlertDestination.Email, url }]),
  );
  expect(saved).toEqual([]);
  const save = dialog.getByRole('button', { name: 'Save' });
  await waitFor(() => expect(save).toBeEnabled());
  await user.click(save);
  await waitFor(() =>
    expect(saved).toEqual([
      { name: 'Operations email', alertDestination: AlertDestination.Email, url, isActive: true },
    ]),
  );
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(await screen.findByText('Operations email')).toBeVisible();
});
