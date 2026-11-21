import { act, screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useEffect } from 'react';
import type { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { InstanceSchedules } from './schedules';

const live = vi.hoisted(() => ({
  changed: undefined as undefined | (() => void),
  invoke: vi.fn(),
}));
vi.mock('@/hooks/useRealtimeGroup', () => ({
  useRealtimeGroup: ({
    setupEventListeners,
    removeEventListeners,
    onJoinedGroup,
  }: Parameters<typeof useRealtimeGroup>[0]) => {
    useEffect(() => {
      const connection = {
        on: (_: string, fn: () => void) => {
          live.changed = fn;
        },
        off: () => {
          live.changed = undefined;
        },
        invoke: live.invoke,
      } as unknown as RealtimeConnection;
      setupEventListeners(connection);
      onJoinedGroup?.(connection);
      return () => removeEventListeners?.(connection);
    }, [setupEventListeners, removeEventListeners, onJoinedGroup]);
  },
}));
beforeEach(() => {
  live.invoke.mockResolvedValue(undefined);
});
const url = 'http://localhost/api/v1/instance/schedules';
it('keeps the active filter and page while a committed change refreshes the catalog', async () => {
  let renamed = false;
  const seen: string[] = [];
  server.use(
    http.get(url, ({ request }) => {
      const params = new URL(request.url).searchParams;
      const after = params.get('after');
      seen.push(params.toString());
      return HttpResponse.json({
        items: [
          {
            purpose: 'Script',
            name: after ? (renamed ? 'Renamed' : 'Second') : 'Nightly cleanup',
            ownerId: after ? 'second' : 'first',
            cron: '0 3 * * *',
            timeZone: 'Europe/Paris',
            enabled: false,
            systemManaged: false,
          },
        ],
        nextCursor: after ? null : 'Script:first',
      });
    }),
  );
  const { user } = renderCitadel(<InstanceSchedules />);
  expect(await screen.findByRole('link', { name: 'Nightly cleanup' })).toHaveAttribute('href', '/scripts/edit/first');
  expect(screen.getByText('Europe/Paris')).toBeVisible();
  expect(screen.getByText('Disabled')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Schedule filter' }));
  await user.click(screen.getByRole('option', { name: 'Script · Run script' }));
  await user.click(await screen.findByRole('button', { name: 'Next' }));
  expect(await screen.findByText('Second')).toBeVisible();
  renamed = true;
  await act(async () => live.changed?.());
  expect(await screen.findByText('Renamed')).toBeVisible();
  expect(seen.at(-1)).toContain('purpose=Script');
  expect(seen.at(-1)).toContain('after=Script%3Afirst');
  expect(live.invoke).toHaveBeenCalledWith('StartInstanceSchedules');
});
it('shows unavailable separately from an empty catalog', async () => {
  server.use(http.get(url, () => new HttpResponse(null, { status: 503 })));
  renderCitadel(<InstanceSchedules />);
  expect(await screen.findByText('Schedule catalog unavailable.')).toBeVisible();
  expect(screen.queryByText('No schedules match this filter.')).not.toBeInTheDocument();
});

it('keeps a system-managed schedule visible when its next deadline is unavailable', async () => {
  server.use(
    http.get(url, () =>
      HttpResponse.json({
        items: [
          {
            purpose: 'StackImages',
            ownerId: 'stack-id',
            name: 'Payments',
            enabled: true,
            systemManaged: true,
            intervalSeconds: 7200,
            nextAt: null,
            reason: 'Platform is offline.',
          },
        ],
        nextCursor: null,
      }),
    ),
  );
  renderCitadel(<InstanceSchedules />);
  expect(await screen.findByRole('link', { name: 'Payments' })).toHaveAttribute('href', '/stacks/edit/stack-id');
  expect(screen.getByText('System managed')).toBeVisible();
  expect(screen.getByText('Every 2 hours')).toBeVisible();
  expect(screen.getByText('Unavailable')).toBeVisible();
  expect(screen.getByText('Platform is offline.')).toBeVisible();
});
