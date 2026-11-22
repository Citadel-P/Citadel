import { act, screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useEffect } from 'react';
import type { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { InstanceSchedules } from './schedules';

const live = vi.hoisted(() => ({
  changed: undefined as undefined | ((snapshot: { catalogChanged: boolean; activeBatches: string[] }) => void),
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
    return { isConnected: true, isLoading: false, isCapacityLimited: false };
  },
}));
beforeEach(() => {
  live.invoke.mockResolvedValue(undefined);
  server.use(http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json({})));
});
const url = 'http://localhost/api/v1/instance/schedules';
function groupCounts(counts: Record<string, number>) {
  server.use(
    http.get(url + '/groups', () =>
      HttpResponse.json(
        Object.entries(counts).map(([purpose, count]) => ({
          purpose,
          count,
          resource:
            (
              {
                Install: 'instance',
                Release: 'instance',
                StackImages: 'stack',
                StackGit: 'stack',
                Git: 'git-repository',
              } as Record<string, string>
            )[purpose] ?? purpose.toLowerCase(),
        })),
      ),
    ),
  );
}

it('keeps the active filter and page while a committed change refreshes the catalog', async () => {
  groupCounts({ Script: 60 });
  let renamed = false;
  const seen: string[] = [];
  server.use(
    http.get(url, ({ request }) => {
      const params = new URL(request.url).searchParams;
      const secondPage = params.get('page') === '2';
      seen.push(params.toString());
      return HttpResponse.json({
        items: [
          {
            purpose: 'Script',
            name: secondPage ? (renamed ? 'Renamed' : 'Second') : 'Nightly cleanup',
            ownerId: secondPage ? 'second' : 'first',
            cron: '0 3 * * *',
            timeZone: 'Europe/Paris',
            activeRunStatus: renamed ? 'Running' : null,
            enabled: false,
            systemManaged: false,
          },
        ],
      });
    }),
  );
  const { user } = renderCitadel(<InstanceSchedules />);
  await user.click(await screen.findByRole('button', { name: 'Script schedules' }));
  expect(await screen.findByRole('link', { name: 'Nightly cleanup' })).toHaveAttribute('href', '/scripts/edit/first');
  expect(screen.getByText('Europe/Paris')).toBeVisible();
  expect(screen.getByText('Disabled')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Resource Filter' }));
  await user.click(screen.getByRole('option', { name: 'Script' }));
  await user.keyboard('{Escape}');
  await user.click(await screen.findByRole('link', { name: 'Go to next page' }));
  expect(await screen.findByText('Second')).toBeVisible();
  renamed = true;
  await act(async () => live.changed?.({ catalogChanged: true, activeBatches: [] }));
  expect(await screen.findByText('Renamed')).toBeVisible();
  expect(screen.getByText('Running')).toBeVisible();
  expect(screen.getByRole('link', { name: '2', current: 'page' })).toBeVisible();
  expect(seen.at(-1)).toContain('resource=script');
  expect(seen.at(-1)).toContain('page=2');
  expect(seen.at(-1)).toContain('pageSize=50');
  await user.type(screen.getByRole('searchbox'), 'nightly');
  await waitFor(() => {
    const last = new URLSearchParams(seen.at(-1));
    expect(last.get('search')).toBe('nightly');
    expect(last.get('resource')).toBe('script');
    expect(last.has('page')).toBe(false);
  });
  expect(live.invoke).toHaveBeenCalledWith('StartInstanceSchedules');
});
it('reuses the instance page when its group is closed and reopened', async () => {
  groupCounts({ Install: 1, Release: 1 });
  const reads: string[] = [];
  server.use(
    http.get(url, ({ request }) => {
      reads.push(new URL(request.url).search);
      return HttpResponse.json({ items: [{ purpose: 'Release', ownerId: 'core', name: 'Core release checks' }] });
    }),
  );
  const { user } = renderCitadel(<InstanceSchedules />);
  const group = await screen.findByRole('button', { name: 'Instance schedules' });
  await user.click(group);
  expect(await screen.findByRole('link', { name: 'Core release checks' })).toHaveAttribute('href', '/updates');
  await user.click(group);
  await user.click(group);
  expect(await screen.findByText('Core release checks')).toBeVisible();
  expect(screen.queryByRole('img')).not.toBeInTheDocument();
  expect(reads).toEqual(['?resource=instance&pageSize=50']);
  await user.click(group);
  // Until the first realtime snapshot arrives, reopening can reconcile an old page.
  const clock = vi.spyOn(Date, 'now').mockReturnValue(Date.now() + 60_000);
  try {
    await user.click(group);
    await waitFor(() => expect(reads).toHaveLength(2));
    await act(async () => live.changed?.({ catalogChanged: false, activeBatches: [] }));
    await user.click(group);
    clock.mockReturnValue(Date.now() + 60_000);
    await user.click(group);
    expect(await screen.findByText('Core release checks')).toBeVisible();
    expect(reads).toHaveLength(2);
  } finally {
    clock.mockRestore();
  }
});

it('shows script enablement beside its name even when its schedule is disabled', async () => {
  groupCounts({ Script: 1 });
  let resourceStatus = 'Enabled';
  server.use(
    http.get(url, () =>
      HttpResponse.json({
        items: [{ purpose: 'Script', ownerId: 'prune', name: 'Prune images', resourceStatus, enabled: false }],
      }),
    ),
  );
  const { user } = renderCitadel(<InstanceSchedules />);
  await user.click(await screen.findByRole('button', { name: 'Script schedules' }));
  const row = within(await screen.findByRole('row', { name: /Prune images/ }));
  expect(row.getByRole('img', { name: 'Script enabled' })).toBeVisible();
  expect(row.getByText('Disabled')).toBeVisible();
  resourceStatus = 'Disabled';
  await act(async () => live.changed?.({ catalogChanged: true, activeBatches: [] }));
  expect(await row.findByRole('img', { name: 'Script disabled' })).toBeVisible();
  expect(row.getByText('Disabled')).toBeVisible();
});

it('shows unavailable separately from an empty catalog', async () => {
  server.use(http.get(url + '/groups', () => new HttpResponse(null, { status: 503 })));
  renderCitadel(<InstanceSchedules />);
  expect(await screen.findByText('Schedule catalog unavailable.')).toBeVisible();
  expect(screen.queryByText('No schedules found.')).not.toBeInTheDocument();
});

it('shows the reason a system-managed schedule is blocked', async () => {
  groupCounts({ StackImages: 1 });
  server.use(
    http.get(url, () =>
      HttpResponse.json({
        items: [
          {
            purpose: 'StackImages',
            ownerId: 'stack-id',
            name: 'Payments',
            needsAttention: true,
            enabled: true,
            systemManaged: true,
            intervalSeconds: 7200,
            nextAt: null,
            reason: 'Platform is offline.',
          },
        ],
      }),
    ),
  );
  const { user } = renderCitadel(<InstanceSchedules />);
  await user.click(await screen.findByRole('button', { name: 'Stack schedules' }));
  expect(await screen.findByRole('link', { name: 'Payments' })).toHaveAttribute('href', '/stacks/edit/stack-id');
  expect(screen.getByText('System managed')).toBeVisible();
  expect(screen.getByText('Every 2 hours')).toBeVisible();
  expect(screen.getByText('Blocked')).toBeVisible();
  expect(screen.getByText('Platform is offline.')).toBeVisible();
});

it('groups both Stack purposes together and keeps their resource links and timestamps', async () => {
  groupCounts({ StackImages: 1, StackGit: 1 });
  const nextAt = '2026-11-23T15:00:00Z';
  const seen: string[] = [];
  server.use(
    http.get(url, ({ request }) => {
      const resource = new URL(request.url).searchParams.get('resource');
      seen.push(resource ?? '');
      return HttpResponse.json({
        items: ['StackImages', 'StackGit'].map((purpose) => ({
          name: 'Storefront',
          purpose,
          ownerId: 'stack',
          enabled: true,
          systemManaged: true,
          intervalSeconds: 7200,
          nextAt,
        })),
      });
    }),
  );
  const { user } = renderCitadel(<InstanceSchedules />);
  const group = await screen.findByRole('button', { name: 'Stack schedules' });
  expect(group).toHaveAttribute('aria-expanded', 'false');
  await user.click(screen.getByRole('button', { name: 'Resource Filter' }));
  await user.click(screen.getByRole('option', { name: 'Stack' }));
  await user.keyboard('{Escape}');
  expect(await screen.findByText('Reconcile Git changes')).toBeVisible();
  expect(screen.getByText('Check image updates')).toBeVisible();
  expect(screen.getAllByRole('link', { name: 'Storefront' })).toHaveLength(2);
  expect(document.querySelectorAll('time[datetime="2026-11-23T15:00:00.000Z"]')).toHaveLength(2);
  expect(seen.at(-1)).toBe('stack');
  expect(screen.queryByRole('button', { name: 'Purpose Filter' })).not.toBeInTheDocument();
  const reads = seen.length;
  await act(async () => live.changed?.({ catalogChanged: false, activeBatches: ['StackGit'] }));
  expect(screen.getByText('Reconcile Git changes · Running')).toBeVisible();
  await act(async () => live.changed?.({ catalogChanged: false, activeBatches: [] }));
  expect(screen.queryByText('Reconcile Git changes · Running')).not.toBeInTheDocument();
  expect(seen).toHaveLength(reads);
});

it('pages images and scripts independently and only reads expanded groups', async () => {
  groupCounts({ Image: 200, Script: 75 });
  const reads: string[] = [];
  server.use(
    http.get(url, ({ request }) => {
      const params = new URL(request.url).searchParams;
      const purpose = params.get('resource') === 'image' ? 'Image' : 'Script';
      const size = Number(params.get('pageSize') ?? 25);
      const offset = (Number(params.get('page') ?? 1) - 1) * size;
      reads.push(`${purpose}:${offset}`);
      const total = purpose === 'Image' ? 200 : 75;
      return HttpResponse.json({
        items: Array.from({ length: Math.min(size, total - offset) }, (_, i) => ({
          purpose,
          ownerId: String(offset + i + 1),
          name: `${purpose} ${offset + i + 1}`,
          enabled: true,
          systemManaged: purpose === 'Image',
        })),
      });
    }),
  );
  const { user } = renderCitadel(<InstanceSchedules />);
  const images = await screen.findByRole('button', { name: 'Image schedules' });
  const scripts = screen.getByRole('button', { name: 'Script schedules' });
  expect(screen.getByText('200 schedules')).toBeVisible();
  expect(reads).toEqual([]);
  await user.click(images);
  expect(await screen.findByText('Image 15')).toBeVisible();
  expect(reads).toEqual(['Image:0']);
  await user.click(scripts);
  await screen.findByText('Script 15');
  const pager = (name: string) => within(screen.getByRole('navigation', { name: name + ' schedule pages' }));
  await user.click(pager('Image').getByRole('link', { name: '4' }));
  expect(await screen.findByText('Image 151')).toBeVisible();
  expect(pager('Script').getByRole('link', { name: '1', current: 'page' })).toBeVisible();
  expect(screen.queryByText('Image 1')).toBeNull();
  expect(screen.getByText('Script 1')).toBeVisible();
  expect(reads).toEqual(['Image:0', 'Script:0', 'Image:150']);
  await user.click(pager('Script').getByRole('link', { name: 'Go to next page' }));
  await screen.findByText('Script 75');
  expect(pager('Image').getByRole('link', { name: '4', current: 'page' })).toBeVisible();
  const readCount = reads.length;
  await act(async () => live.changed?.({ catalogChanged: false, activeBatches: [] }));
  await user.click(images);
  await user.click(images);
  expect(await screen.findByText('Image 151')).toBeVisible();
  expect(reads).toHaveLength(readCount);
  await user.click(images);
  expect(screen.queryByRole('navigation', { name: 'Image schedule pages' })).toBeNull();
  await act(async () => live.changed?.({ catalogChanged: true, activeBatches: [] }));
  await waitFor(() => expect(reads.at(-1)).toBe('Script:50'));
  await user.click(images);
  await waitFor(() => expect(reads.slice(readCount)).toEqual(['Script:50', 'Image:150']));
  expect(await screen.findByText('Image 151')).toBeVisible();
  await user.click(pager('Image').getByRole('link', { name: '1' }));
  expect(await screen.findByText('Image 1')).toBeVisible();
  expect(screen.getByText('Script 51')).toBeVisible();
});

it('finds confirmed blockers outside the first page and links the actual scheduled run', async () => {
  let attentionRequest = false;
  server.use(
    http.get(url + '/groups', () =>
      HttpResponse.json([
        { purpose: 'Script', resource: 'script', count: 29, attention: 1 },
        { purpose: 'Image', resource: 'image', count: 150, attention: 0 },
      ]),
    ),
    http.get(url, ({ request }) => {
      const params = new URL(request.url).searchParams;
      attentionRequest = params.get('attentionOnly') === 'true' && params.get('resource') === 'script';
      return HttpResponse.json({
        items: attentionRequest
          ? [
              {
                purpose: 'Script',
                ownerId: 'job',
                name: 'Cleanup',
                enabled: true,
                needsAttention: true,
                reason: 'The script timeout exceeds the limit configured for this Core.',
                latestRun: { id: 'last', status: 'Failed', queuedAt: '2026-11-20T00:00:00Z' },
              },
            ]
          : [],
      });
    }),
  );
  const { user } = renderCitadel(<InstanceSchedules />);
  expect(await screen.findByText('1 blocked')).toBeVisible();
  await user.click(screen.getByRole('switch', { name: 'Needs attention' }));
  expect(await screen.findByText('The script timeout exceeds the limit configured for this Core.')).toBeVisible();
  expect(attentionRequest).toBe(true);
  expect(screen.queryByRole('button', { name: 'Image schedules' })).toBeNull();
  expect(screen.getByText('Failed')).toBeVisible();
  expect(screen.getByRole('link', { name: 'View scheduled run for Cleanup' })).toHaveAttribute(
    'href',
    '/scripts/edit/job?runId=last#runs',
  );
});

it('keeps a busy repository with an unknown deadline out of Needs attention', async () => {
  groupCounts({ Git: 1 });
  server.use(
    http.get(url, () =>
      HttpResponse.json({
        items: [
          {
            purpose: 'Git',
            name: 'Storefront',
            ownerId: 'repo',
            enabled: true,
            needsAttention: false,
            nextAt: null,
            reason: 'Repository synchronization is busy.',
          },
        ],
      }),
    ),
  );
  const { user } = renderCitadel(<InstanceSchedules />);
  await user.click(await screen.findByRole('button', { name: 'Git repository schedules' }));
  expect(await screen.findByText('Next run unknown')).toBeVisible();
  expect(screen.getByText('Repository synchronization is busy.')).toBeVisible();
  await user.click(screen.getByRole('switch', { name: 'Needs attention' }));
  expect(await screen.findByText('No blocked schedules match your filters.')).toBeVisible();
});
