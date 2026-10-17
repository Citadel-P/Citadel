import { act, renderHook } from '@testing-library/react';
import type { NetworksResponse, NetworkView } from '@/api/generated/api.types';
import type { DockerDaemonListeners } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useNetworksGroup } from './useNetworksGroup';

const state = vi.hoisted(() => ({
  data: undefined as { data: NetworksResponse } | undefined,
  listeners: undefined as DockerDaemonListeners | undefined,
  refetch: vi.fn(),
  isFetching: false,
  dataUpdatedAt: 1,
}));
vi.mock('@/lib/hooks', () => ({
  useRead: () => ({
    data: state.data,
    refetch: state.refetch,
    isFetching: state.isFetching,
    dataUpdatedAt: state.dataUpdatedAt,
  }),
}));
vi.mock('@/features/platforms/hooks/useDockerDaemonGroup', () => ({
  useDockerDaemonGroup: (_: unknown, listeners: DockerDaemonListeners) => {
    state.listeners = listeners;
  },
}));
const response = (id: string) => ({
  data: {
    networks: [{ id, dockerNodeId: null } as NetworkView],
    capabilities: { canRead: true, canWrite: true, canExecute: true },
  },
});

it('coalesces usage changes for an open workload map and cancels refreshes on close', () => {
  vi.useFakeTimers();
  state.refetch.mockClear();
  state.data = response('network');
  const { unmount } = renderHook(() => useNetworksGroup('p', true));
  act(() => {
    state.listeners?.onNetworkUsageChanged?.('other-platform');
    vi.advanceTimersByTime(300);
  });
  expect(state.refetch).not.toHaveBeenCalled();
  act(() => {
    state.listeners?.onNetworkUsageChanged?.('p');
    state.listeners?.onNetworkUsageChanged?.('p');
    vi.advanceTimersByTime(300);
  });
  expect(state.refetch).toHaveBeenCalledTimes(1);
  act(() => state.listeners?.onNetworkUsageChanged?.('p'));
  unmount();
  act(() => vi.advanceTimersByTime(300));
  expect(state.refetch).toHaveBeenCalledTimes(1);
  vi.useRealTimers();
});

it('does not carry a previous platform’s node snapshot into another platform', () => {
  state.data = response('manager-a');
  const { result, rerender } = renderHook(({ platformId }) => useNetworksGroup(platformId), {
    initialProps: { platformId: 'a' },
  });
  act(() =>
    state.listeners?.onSwarmNodeLocalResourcesUpdated?.({
      platformId: 'a',
      networks: [{ id: 'worker-a', dockerNodeId: 'node-a' } as NetworkView],
    }),
  );
  expect(result.current.networks?.networks.map((network) => network.id)).toEqual(['manager-a', 'worker-a']);
  state.data = undefined;
  rerender({ platformId: 'b' });
  expect(result.current.networks).toBeUndefined();
  state.data = response('manager-b');
  rerender({ platformId: 'b' });
  expect(result.current.networks?.networks.map((network) => network.id)).toEqual(['manager-b']);
  act(() =>
    state.listeners?.onSwarmNodeLocalResourcesUpdated?.({
      platformId: 'a',
      networks: [{ id: 'late-a', dockerNodeId: 'node-a' } as NetworkView],
    }),
  );
  expect(result.current.networks?.networks.map((network) => network.id)).toEqual(['manager-b']);
  state.data = response('manager-a-new');
  rerender({ platformId: 'a' });
  expect(result.current.networks?.networks.map((network) => network.id)).toEqual(['manager-a-new']);
});

it('accepts fresh worker state from HTTP instead of retaining a pre-request snapshot', () => {
  state.isFetching = false;
  state.data = response('manager');
  const { result, rerender } = renderHook(() => useNetworksGroup('p', true));
  const worker = { id: 'worker-network', dockerNodeId: 'worker', inUse: true } as NetworkView;
  act(() => state.listeners?.onSwarmNodeLocalResourcesUpdated?.({ platformId: 'p', networks: [worker] }));
  state.isFetching = true;
  rerender();
  state.isFetching = false;
  state.data = {
    ...response('manager'),
    data: { ...response('manager').data, networks: [{ ...worker, inUse: false }] },
  };
  state.dataUpdatedAt++;
  rerender();
  expect(result.current.networks?.networks).toEqual([{ ...worker, inUse: false }]);
});

it('preserves worker observations received during an in-flight HTTP read', () => {
  state.isFetching = false;
  state.data = response('manager');
  const { result, rerender } = renderHook(() => useNetworksGroup('p', true));
  state.isFetching = true;
  rerender();
  const worker = { id: 'new-worker-network', dockerNodeId: 'worker' } as NetworkView;
  act(() => state.listeners?.onSwarmNodeLocalResourcesUpdated?.({ platformId: 'p', networks: [worker] }));
  state.isFetching = false;
  state.data = response('manager');
  state.dataUpdatedAt++;
  rerender();
  expect(result.current.networks?.networks.map((network) => network.id)).toEqual(['manager', 'new-worker-network']);
});
