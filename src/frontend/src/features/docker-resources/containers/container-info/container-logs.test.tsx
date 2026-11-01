import { renderCitadel } from '@/test/render-citadel';
import { act, cleanup, screen } from '@testing-library/react';
import { ContainerLogs, DeploymentLogs, StackLogs } from './container-logs';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';

const { logViewerMock, groupMock } = vi.hoisted(() => ({
  logViewerMock: vi.fn(),
  groupMock: vi.fn(() => ({ isLoading: false })),
}));

vi.mock('@/hooks/useRealtimeGroup', () => ({
  useRealtimeGroup: groupMock,
}));

vi.mock('@/components/custom/log-viewer', () => ({
  LogViewer: (props: { containerFilters?: string[]; enableContainerFilter?: boolean }) => {
    logViewerMock(props);
    return <div>Log viewer</div>;
  },
}));

describe('StackLogs', () => {
  beforeEach(() => logViewerMock.mockClear());

  it('keeps container filtering enabled while container names are discovered from streamed log prefixes', () => {
    renderCitadel(<StackLogs stackId="stack-1" containers={[]} />);

    expect(screen.getByText('Log viewer')).toBeVisible();
    expect(logViewerMock).toHaveBeenCalledWith(
      expect.objectContaining({
        containerFilters: [],
        enableContainerFilter: true,
        emptyMessage: 'Waiting for logs…',
      }),
    );
  });

  it('passes all known container names to the shared multi-select filter', () => {
    renderCitadel(<StackLogs stackId="stack-1" containers={['web.1', 'worker.1']} />);

    expect(logViewerMock).toHaveBeenCalledWith(
      expect.objectContaining({
        containerFilters: ['web.1', 'worker.1'],
        enableContainerFilter: true,
      }),
    );
  });
});

describe('DeploymentLogs', () => {
  beforeEach(() => {
    logViewerMock.mockClear();
    groupMock.mockClear();
  });

  it('reports missing containers instead of waiting for a stream that cannot start', () => {
    renderCitadel(<DeploymentLogs deploymentId="deployment-1" containerId={undefined} />);
    expect(logViewerMock).toHaveBeenLastCalledWith(
      expect.objectContaining({
        emptyMessage: 'No container available for logs.',
      }),
    );
  });

  it('displays stream startup failures instead of waiting indefinitely', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    renderCitadel(<DeploymentLogs deploymentId="deployment-1" containerId="abcdef123456" />);
    const options = vi.mocked(useRealtimeGroup).mock.lastCall![0];
    await act(async () =>
      options.onJoinedGroup?.({
        invoke: vi.fn().mockRejectedValue(new Error('Unavailable')),
      } as unknown as RealtimeConnection),
    );
    expect(logViewerMock).toHaveBeenLastCalledWith(
      expect.objectContaining({
        emptyMessage: 'No logs received.',
      }),
    );
    expect(screen.getByRole('alert')).toHaveTextContent('Unable to load container logs.');
    consoleError.mockRestore();
  });
});

describe('ContainerLogs stream failures', () => {
  it('shows failures for its own group while keeping the log viewer mounted', async () => {
    renderCitadel(<ContainerLogs containerId="abcdef123456" />);
    const handlers = new Map<string, (...args: any[]) => void>();
    const hub = {
      on: vi.fn((name, handler) => handlers.set(name, handler)),
      off: vi.fn(),
    } as unknown as RealtimeConnection;
    const options = vi.mocked(useRealtimeGroup).mock.lastCall![0];
    options.setupEventListeners(hub);

    act(() => handlers.get('LogStreamError')?.('container-log:other', 'Other failure'));
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();

    act(() =>
      handlers.get('LogStreamError')?.(
        'container-log:abcdef123456',
        'Log stream stopped. Reopen the Logs tab to retry.',
      ),
    );
    expect(screen.getByRole('alert')).toHaveTextContent('Log stream stopped.');
    expect(screen.getByText('Log viewer')).toBeVisible();
    options.removeEventListeners?.(hub);
    expect(hub.off).toHaveBeenCalledWith('LogStreamError', handlers.get('LogStreamError'));
  });
});

describe('container log history replay', () => {
  const first = '2026-10-29T16:59:39.884427123Z WARN sample failed\n';
  const second = '2026-10-29T17:13:06.891434123Z WARN update check failed\n';
  const latestLogs = () => logViewerMock.mock.lastCall![0].logs as { timestamp: string; message: string }[];

  const subscribe = () => {
    const handlers = new Map<string, (data: Uint8Array) => void>();
    const hub = {
      on: vi.fn((name, handler) => handlers.set(name, handler)),
      off: vi.fn((name) => handlers.delete(name)),
    } as unknown as RealtimeConnection;
    const options = vi.mocked(useRealtimeGroup).mock.lastCall![0];
    act(() => options.setupEventListeners(hub));
    return {
      emit(text: string, event = 'SendContainerLogs', flush = true) {
        act(() => {
          handlers.get(event)?.(new TextEncoder().encode(text));
          if (flush) vi.advanceTimersByTime(50);
        });
      },
      disconnect() {
        act(() => options.removeEventListeners?.(hub));
      },
    };
  };

  beforeEach(() => {
    vi.useFakeTimers();
    logViewerMock.mockClear();
    groupMock.mockClear();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it('does not multiply cached history when the container tab is opened four times', () => {
    for (let visit = 0; visit < 4; visit++) {
      const view = renderCitadel(<ContainerLogs containerId="replay-remount" />);
      const stream = subscribe();
      stream.emit(first + second);
      expect(latestLogs().map((log) => log.message)).toEqual(['WARN sample failed', 'WARN update check failed']);
      stream.disconnect();
      view.unmount();
    }
  });

  it('reconciles replay across batches on reconnect without losing repeated occurrences', () => {
    renderCitadel(<ContainerLogs containerId="replay-reconnect" />);
    const initial = subscribe();
    initial.emit(first + first);
    expect(latestLogs()).toHaveLength(2);
    initial.disconnect();

    const reconnected = subscribe();
    reconnected.emit(first, 'SendContainerLogsBatch');
    reconnected.emit(first + second, 'SendContainerLogsBatch');
    expect(latestLogs().map((log) => log.message)).toEqual([
      'WARN sample failed',
      'WARN sample failed',
      'WARN update check failed',
    ]);
    reconnected.emit(first);
    expect(latestLogs()).toHaveLength(4);
    reconnected.disconnect();
  });

  it('preserves identical live lines and messages with different Docker timestamps', () => {
    renderCitadel(<ContainerLogs containerId="replay-live" />);
    const stream = subscribe();
    stream.emit(first);
    stream.emit(first);
    stream.emit(first.replace('884427123Z', '884427124Z'));
    expect(latestLogs()).toHaveLength(3);
    stream.disconnect();
  });

  it('reconciles each stack member even when replay arrives after another member has new output', () => {
    renderCitadel(<StackLogs stackId="replay-stack" />);
    const web = first.replace('WARN', '[web] WARN');
    const redis = first.replace('WARN', '[redis] WARN');
    const initial = subscribe();
    initial.emit(web + redis, 'SendStackLogs');
    initial.disconnect();

    const reconnected = subscribe();
    reconnected.emit(second.replace('WARN', '[web] WARN'), 'SendStackLogs');
    reconnected.emit(redis + web, 'SendStackLogsBatch');
    expect(latestLogs()).toHaveLength(3);
    reconnected.disconnect();
  });

  it('commits buffered lines on detach before a new viewer reads the cache', () => {
    const view = renderCitadel(<ContainerLogs containerId="replay-buffer" />);
    const initial = subscribe();
    initial.emit(first, 'SendContainerLogs', false);
    initial.disconnect();
    view.unmount();

    renderCitadel(<ContainerLogs containerId="replay-buffer" />);
    expect(latestLogs()).toHaveLength(1);
    const reopened = subscribe();
    reopened.emit(first + second);
    expect(latestLogs()).toHaveLength(2);
    act(() => vi.advanceTimersByTime(100));
    expect(latestLogs()).toHaveLength(2);
    reopened.disconnect();
  });

  it('clears history and replay counts together', () => {
    renderCitadel(<ContainerLogs containerId="replay-clear" />);
    const initial = subscribe();
    initial.emit(first);
    initial.disconnect();
    const reconnected = subscribe();
    act(() => logViewerMock.mock.lastCall![0].onClear());
    expect(latestLogs()).toHaveLength(0);
    reconnected.emit(first);
    expect(latestLogs()).toHaveLength(1);
    reconnected.disconnect();
  });

  it('keeps histories isolated when switching container groups', () => {
    const view = renderCitadel(<ContainerLogs containerId="aaaaaaaaaaaa" />);
    const initial = subscribe();
    initial.emit(first);
    initial.disconnect();
    view.rerender(<ContainerLogs containerId="bbbbbbbbbbbb" />);
    expect(latestLogs()).toHaveLength(0);
    const next = subscribe();
    next.emit(first);
    expect(latestLogs()).toHaveLength(1);
    next.disconnect();
  });

  it('keeps the 5000-line limit while merging cached and replayed history', () => {
    renderCitadel(<ContainerLogs containerId="replay-limit" />);
    const initial = subscribe();
    initial.emit(Array.from({ length: 5001 }, (_, i) => first.replace('sample failed', `sample ${i}`)).join(''));
    expect(latestLogs()).toHaveLength(5000);
    expect(latestLogs()[0].message).toBe('WARN sample 1');
    initial.disconnect();
    const reconnected = subscribe();
    reconnected.emit(first.replace('sample failed', 'sample 5000') + second);
    expect(latestLogs()).toHaveLength(5000);
    expect(latestLogs()[0].message).toBe('WARN sample 2');
    reconnected.disconnect();
  });
});
