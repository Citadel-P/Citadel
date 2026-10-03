import { renderCitadel } from '@/test/render-citadel';
import { act, screen } from '@testing-library/react';
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

vi.mock('@/components/custom/common', () => ({
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
