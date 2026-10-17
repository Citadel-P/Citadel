import { act, renderHook } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { ContainerStateStatus, type NetworkView } from '@/api/generated/api.types';
import type { DockerDaemonListeners } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useNetworkInspection } from './useNetworkInspection';

const { state, read } = vi.hoisted(() => ({
  state: { listeners: undefined as DockerDaemonListeners | undefined },
  read: vi.fn(() => ({})),
}));
vi.mock('@/lib/hooks', () => ({ useRead: read }));
vi.mock('@/features/platforms/hooks/useDockerDaemonGroup', () => ({
  useDockerDaemonGroup: (_: unknown, listeners: DockerDaemonListeners) => {
    state.listeners = listeners;
  },
}));

it('coalesces relevant realtime updates and cancels pending refresh on close', () => {
  vi.useFakeTimers();
  try {
    const client = new QueryClient();
    const invalidate = vi.spyOn(client, 'invalidateQueries');
    const network = { id: 'network-a', dockerNodeId: null, capabilities: null } as NetworkView;
    const { unmount } = renderHook(() => useNetworkInspection(network, 'platform-a'), {
      wrapper: ({ children }: { children: ReactNode }) => (
        <QueryClientProvider client={client}>{children}</QueryClientProvider>
      ),
    });
    act(() => {
      state.listeners?.onNetworkEvent?.({
        actorId: 'other',
        eventType: 'create',
        network: { id: 'other' } as NetworkView,
      });
      vi.advanceTimersByTime(300);
    });
    expect(invalidate).not.toHaveBeenCalled();
    act(() => {
      for (let i = 0; i < 4; i++)
        state.listeners?.onNetworkEvent?.({ actorId: network.id, eventType: 'create', network });
      vi.advanceTimersByTime(300);
    });
    expect(invalidate).toHaveBeenCalledTimes(1);
    expect(invalidate).toHaveBeenCalledWith({
      queryKey: [
        'inspectNetwork',
        { platformId: 'platform-a', networkId: 'network-a', query: { dockerNodeId: undefined } },
      ],
      exact: true,
    });
    act(() => state.listeners?.onNetworkEvent?.({ actorId: network.id, eventType: 'destroy', network }));
    unmount();
    act(() => vi.advanceTimersByTime(300));
    expect(invalidate).toHaveBeenCalledTimes(1);
  } finally {
    vi.useRealTimers();
  }
});

it('refreshes for attached container state changes only and coalesces patches', () => {
  vi.useFakeTimers();
  try {
    const client = new QueryClient();
    const invalidate = vi.spyOn(client, 'invalidateQueries');
    const network = {
      id: 'network-a',
      capabilities: null,
      usage: { entries: [{ relationship: 'Attachment', containerId: 'attached' }] },
    } as NetworkView;
    const { unmount } = renderHook(() => useNetworkInspection(network, 'platform-a'), {
      wrapper: ({ children }: { children: ReactNode }) => (
        <QueryClientProvider client={client}>{children}</QueryClientProvider>
      ),
    });
    act(() => {
      state.listeners?.onContainerStateChange?.([
        { id: 'other', containerId: 'docker-other', state: ContainerStateStatus.Paused },
      ]);
      vi.advanceTimersByTime(300);
    });
    expect(invalidate).not.toHaveBeenCalled();
    act(() => {
      for (let i = 0; i < 4; i++)
        state.listeners?.onContainerStateChange?.([
          { id: 'attached', containerId: 'docker-id', state: ContainerStateStatus.Paused },
        ]);
      vi.advanceTimersByTime(300);
    });
    expect(invalidate).toHaveBeenCalledTimes(1);
    unmount();
  } finally {
    vi.useRealTimers();
  }
});
