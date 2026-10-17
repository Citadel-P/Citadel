import { act, screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import {
  BackupCoverageStatus as Status,
  type ResourceBackupSummaryView,
  type StackView,
} from '@/api/generated/api.types';
import type { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { StackBackupCell } from './backup-cell';

const realtime = vi.hoisted(() => ({
  options: undefined as Parameters<typeof useRealtimeGroup>[0] | undefined,
  connected: true,
}));
vi.mock('@/hooks/useRealtimeGroup', () => ({
  useRealtimeGroup: (options: Parameters<typeof useRealtimeGroup>[0]) => {
    realtime.options = options;
    return { isConnected: realtime.connected, isLoading: false };
  },
}));
beforeEach(() => {
  realtime.connected = true;
});

const coverage: ResourceBackupSummaryView = {
  status: Status.Unprotected,
  platformId: 'platform',
  unavailable: false,
  canConfigure: true,
  hasResourcePolicy: false,
  policies: [],
  warnings: [],
  reasons: [],
  volumes: [{ name: 'wordpress-data', includedInResourcePolicy: true, status: Status.Unprotected, policies: [] }],
};
function subscribe() {
  const on = vi.fn();
  const off = vi.fn();
  const connection = { on, off } as unknown as RealtimeConnection;
  realtime.options!.setupEventListeners(connection);
  expect(on).toHaveBeenCalledWith('StackInfoUpdated', expect.any(Function));
  return { receive: on.mock.calls[0][1] as (stack: StackView, action: string) => void, off, connection };
}
const stack = (id = 'wordpress', backupCoverage = coverage) => ({ id, backupCoverage }) as StackView;

it('uses the full stack summary, shared hover card and stack configuration link without HTTP', async () => {
  const requests = vi.fn(() => HttpResponse.json({}));
  server.use(http.get('http://localhost/api/v1/stacks/:id', requests));
  const { user } = renderCitadel(<StackBackupCell stackId="wordpress" />);
  expect(realtime.options?.groupName).toBe('stack:wordpress');
  const { receive, off, connection } = subscribe();
  expect(screen.getByRole('button', { name: 'Backups: Coverage unavailable' })).toBeVisible();
  await act(async () => receive(stack(), 'update'));
  await user.hover(screen.getByRole('button', { name: 'Backups: Volumes unprotected' }));
  expect(await screen.findByRole('dialog', { name: 'Backups' })).toBeVisible();
  expect(screen.getByRole('link', { name: 'wordpress-data' })).toBeVisible();
  const link = new URL(
    screen.getByRole('link', { name: 'Configure backup' }).getAttribute('href')!,
    'http://localhost',
  );
  expect(link.searchParams.get('sourceType')).toBe('Stack');
  expect(link.searchParams.get('stackId')).toBe('wordpress');
  expect(link.searchParams.get('returnTo')).toBe('/stacks/edit/wordpress/');
  await act(async () => receive(stack('wordpress', { ...coverage, status: Status.Failed }), 'update'));
  expect(screen.getByRole('button', { name: 'Backups: Backup failed' })).toBeVisible();
  expect(requests).not.toHaveBeenCalled();
  realtime.options!.removeEventListeners!(connection);
  expect(off).toHaveBeenCalledWith('StackInfoUpdated', receive);
});

it('isolates stack identities and clears unavailable or deleted coverage', async () => {
  const { rerender } = renderCitadel(<StackBackupCell stackId="wordpress" />);
  const first = subscribe();
  await act(async () => first.receive(stack('other'), 'update'));
  expect(screen.getByRole('button', { name: 'Backups: Coverage unavailable' })).toBeVisible();
  await act(async () => first.receive(stack(), 'update'));
  expect(screen.getByRole('button', { name: 'Backups: Volumes unprotected' })).toBeVisible();
  realtime.connected = false;
  rerender(<StackBackupCell stackId="wordpress" />);
  expect(screen.getByRole('button', { name: 'Backups: Coverage unavailable' })).toBeVisible();
  realtime.connected = true;
  rerender(<StackBackupCell stackId="wordpress" />);
  await act(async () =>
    first.receive(stack('wordpress', { ...coverage, status: Status.NotApplicable, volumes: [] }), 'update'),
  );
  expect(screen.getByTitle('No eligible storage')).toHaveTextContent('—');
  rerender(<StackBackupCell stackId="other" />);
  expect(screen.getByRole('button', { name: 'Backups: Coverage unavailable' })).toBeVisible();
  const second = subscribe();
  await act(async () => second.receive(stack('other'), 'update'));
  await act(async () => second.receive(stack('other'), 'delete'));
  expect(screen.getByRole('button', { name: 'Backups: Coverage unavailable' })).toBeVisible();
});
