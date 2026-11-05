import { act, renderHook } from '@testing-library/react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { latestLiveStat } from '@/lib/stats-buckets';
import { useServiceStatsStream } from './useServiceStats';

const { useRealtimeGroupMock } = vi.hoisted(() => ({ useRealtimeGroupMock: vi.fn() }));

vi.mock('@/hooks/useRealtimeGroup', () => ({ useRealtimeGroup: useRealtimeGroupMock }));

describe('useServiceStatsStream', () => {
  beforeEach(() => useRealtimeGroupMock.mockReset());

  it('joins the Platform stream before task container projections are available', () => {
    renderHook(() => useServiceStatsStream('platform-1', []));

    expect(useRealtimeGroupMock).toHaveBeenCalledWith(
      expect.objectContaining({
        groupName: 'containers:platform-1',
        skip: false,
      }),
    );
  });

  it('streams matching statistics without registering an HTTP refresh listener', () => {
    const { result } = renderHook(() => useServiceStatsStream('platform-1', ['container-1']));
    const options = useRealtimeGroupMock.mock.calls[0][0];
    const listeners = new Map<string, (...args: any[]) => void>();
    const connection = {
      on: vi.fn((event: string, handler: (...args: any[]) => void) => listeners.set(event, handler)),
      off: vi.fn(),
    } as unknown as RealtimeConnection;
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

it('counts traffic per Agent container before summing, including separate arrivals and resets', () => {
  useRealtimeGroupMock.mockReset();
  const { result } = renderHook(() => useServiceStatsStream('platform-1', ['a', 'b'], 300));
  const options = useRealtimeGroupMock.mock.calls[0][0];
  let receive: (samples: unknown[]) => void = () => {};
  options.setupEventListeners({
    on: (_event: string, handler: typeof receive) => {
      receive = handler;
    },
  });
  act(() =>
    receive([
      { containerId: 'a', created: 301, rxBytes: 1000, txBytes: 2000, blockReadBytes: 100, blockWriteBytes: 200 },
      { containerId: 'b', created: 301, rxBytes: 3000, txBytes: 4000, blockReadBytes: 300, blockWriteBytes: 400 },
    ]),
  );
  act(() =>
    receive([{ containerId: 'a', created: 311, rxBytes: 10, txBytes: 2005, blockReadBytes: 2, blockWriteBytes: 210 }]),
  );
  act(() =>
    receive([
      { containerId: 'b', created: 311, rxBytes: 3100, txBytes: 4007, blockReadBytes: 310, blockWriteBytes: 420 },
    ]),
  );
  expect(result.current.stats[0]).toMatchObject({
    rxBytesDelta: 110,
    txBytesDelta: 12,
    rxBytesPerSecond: 11,
    txBytesPerSecond: 1.2,
    blockReadBytesDelta: 12,
    blockWriteBytesDelta: 30,
    blockReadBytesPerSecond: 1.2,
    blockWriteBytesPerSecond: 3,
  });
  act(() =>
    receive([
      { containerId: 'b', created: 311, rxBytes: 3100, txBytes: 4007, blockReadBytes: 310, blockWriteBytes: 420 },
    ]),
  );
  expect(result.current.stats[0]).toMatchObject({
    rxBytesDelta: 110,
    txBytesDelta: 12,
    rxBytesPerSecond: 11,
    txBytesPerSecond: 1.2,
    blockReadBytesDelta: 12,
    blockWriteBytesDelta: 30,
    blockReadBytesPerSecond: 1.2,
    blockWriteBytesPerSecond: 3,
  });
});

it('averages each task over its measured time before summing asynchronous arrivals', () => {
  useRealtimeGroupMock.mockReset();
  const { result, rerender } = renderHook(({ ids }) => useServiceStatsStream('p', ids, 300), {
    initialProps: { ids: ['a', 'b'] },
  });
  let receive: (samples: unknown[]) => void = () => {};
  useRealtimeGroupMock.mock.calls[0][0].setupEventListeners({
    on: (_event: string, handler: typeof receive) => {
      receive = handler;
    },
  });
  act(() =>
    receive([
      { containerId: 'a', created: 301, rxBytes: 100, blockReadBytes: 100, cpuUsage: 1 },
      { containerId: 'b', created: 301, rxBytes: 100, blockReadBytes: 100, cpuUsage: 2 },
    ]),
  );
  act(() => receive([{ containerId: 'a', created: 311, rxBytes: 200, blockReadBytes: 200, cpuUsage: 3 }]));
  act(() => receive([{ containerId: 'a', created: 321, rxBytes: 400, blockReadBytes: 400, cpuUsage: 5 }]));
  act(() => receive([{ containerId: 'b', created: 321, rxBytes: 500, blockReadBytes: 500, cpuUsage: 4 }]));
  expect(result.current.stats[0]).toMatchObject({
    rxBytesPerSecond: 35,
    blockReadBytesPerSecond: 35,
    rxBytesDelta: 700,
    blockReadBytesDelta: 700,
    cpuUsage: 6,
  });
  expect(latestLiveStat(result.current.stats)).toMatchObject({ cpuUsage: 9, created: 321 });
  act(() => receive([{ containerId: 'b', created: 321, rxBytes: 500, blockReadBytes: 500, cpuUsage: 4 }]));
  expect(result.current.stats[0]).toMatchObject({ rxBytesPerSecond: 35, rxBytesDelta: 700 });
  rerender({ ids: ['replacement'] });
  expect(result.current.stats).toEqual([]);
});
