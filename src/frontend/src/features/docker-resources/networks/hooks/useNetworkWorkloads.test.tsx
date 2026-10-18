import { act, renderHook } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { ContainerStateStatus } from '@/api/generated/api.types';
import type { DockerDaemonListeners } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useNetworkWorkloads } from './useNetworkWorkloads';

const state = vi.hoisted(() => ({
  listeners: undefined as DockerDaemonListeners | undefined,
  read: vi.fn(() => ({})),
}));
vi.mock('@/lib/hooks', () => ({ useRead: state.read }));
vi.mock('@/features/platforms/hooks/useDockerDaemonGroup', () => ({
  useDockerDaemonGroup: (_: unknown, listeners: DockerDaemonListeners) => {
    state.listeners = listeners;
  },
}));

it('inspects only the selected endpoint, coalesces events and stops on close without polling', () => {
  vi.useFakeTimers();
  try {
    const client = new QueryClient();
    const invalidate = vi.spyOn(client, 'invalidateQueries');
    const { unmount } = renderHook(() => useNetworkWorkloads('platform', 'container', false), {
      wrapper: ({ children }: { children: ReactNode }) => (
        <QueryClientProvider client={client}>{children}</QueryClientProvider>
      ),
    });
    expect(state.read).toHaveBeenCalledWith(
      'listNetworkContainers',
      { id: 'platform' },
      expect.objectContaining({ enabled: false }),
    );
    expect(state.read).toHaveBeenCalledWith(
      'getContainerNetworking',
      { id: 'container' },
      expect.objectContaining({ enabled: true }),
    );
    act(() => {
      state.listeners?.onNetworkUsageChanged?.('other-platform');
      vi.advanceTimersByTime(300);
    });
    expect(invalidate).not.toHaveBeenCalled();
    act(() => {
      for (let i = 0; i < 10; i++) state.listeners?.onNetworkUsageChanged?.('platform');
      vi.advanceTimersByTime(300);
    });
    expect(invalidate).toHaveBeenCalledTimes(1);
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ['getContainerNetworking', { id: 'container' }], exact: true });
    act(() => vi.advanceTimersByTime(60_000));
    expect(invalidate).toHaveBeenCalledTimes(1);
    act(() => {
      state.listeners?.onContainerStateChange?.([
        { id: 'other', containerId: 'docker-other', state: ContainerStateStatus.Exited },
      ]);
      vi.advanceTimersByTime(300);
    });
    expect(invalidate).toHaveBeenCalledTimes(1);
    act(() => {
      state.listeners?.onContainerStateChange?.([
        { id: 'container', containerId: 'docker-selected', state: ContainerStateStatus.Exited },
      ]);
      vi.advanceTimersByTime(300);
    });
    expect(invalidate).toHaveBeenCalledTimes(2);
    act(() => state.listeners?.onNetworkUsageChanged?.('platform'));
    unmount();
    act(() => vi.advanceTimersByTime(300));
    expect(invalidate).toHaveBeenCalledTimes(2);
  } finally {
    vi.useRealTimers();
  }
});
