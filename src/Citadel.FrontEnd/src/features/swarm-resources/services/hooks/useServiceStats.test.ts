import { act, renderHook } from '@testing-library/react';
import { HubConnection } from '@microsoft/signalr';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useServiceStatsStream } from './useServiceStats';

const { useSignalRGroupMock } = vi.hoisted(() => ({ useSignalRGroupMock: vi.fn() }));

vi.mock('@/hooks/useSignalRGroup', () => ({ useSignalRGroup: useSignalRGroupMock }));

describe('useServiceStatsStream', () => {
  beforeEach(() => useSignalRGroupMock.mockReset());

  it('joins the Platform stream before task container projections are available', () => {
    renderHook(() => useServiceStatsStream('platform-1', []));

    expect(useSignalRGroupMock).toHaveBeenCalledWith(
      expect.objectContaining({
        groupName: 'containers:platform-1',
        skip: false,
      }),
    );
  });

  it('streams matching statistics without registering an HTTP refresh listener', () => {
    const { result } = renderHook(() => useServiceStatsStream('platform-1', ['container-1']));
    const options = useSignalRGroupMock.mock.calls[0][0];
    const listeners = new Map<string, (...args: any[]) => void>();
    const connection = {
      on: vi.fn((event: string, handler: (...args: any[]) => void) => listeners.set(event, handler)),
      off: vi.fn(),
    } as unknown as HubConnection;
    options.setupEventListeners(connection);

    act(() => {
      listeners.get('ContainersStatsUpdated')?.([{ containerId: 'container-1', cpuUsage: 2, created: 1 }]);
      listeners.get('ContainersStatsUpdated')?.([{ containerId: 'other-container', cpuUsage: 3, created: 2 }]);
    });

    expect(listeners.has('ContainersInfoUpdated')).toBe(false);
    expect(options.onJoinedGroup).toBeUndefined();
    expect(result.current).toEqual({
      observedContainers: 1,
      stats: [expect.objectContaining({ cpuUsage: 2 })],
    });
  });
});
