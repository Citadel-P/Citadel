import { collectionEventKey, collectionGroup } from './collection-scope';
import { RealtimeConnectionState } from '@/lib/realtime-connection';
import { FakeWebSocket } from '@/test/fakes/websocket';
import { createWebSocketConnection } from './createWebSocketConnection';

function fixture() {
  const sockets: FakeWebSocket[] = [];
  const accessTokenFactory = vi.fn(() => 'session-token');
  const connection = createWebSocketConnection({ baseUrl: 'http://localhost:8000', accessTokenFactory }, () => {
    const socket = new FakeWebSocket();
    sockets.push(socket);
    return socket.asWebSocket();
  });
  return { connection, sockets, accessTokenFactory };
}

async function connect(f: ReturnType<typeof fixture>) {
  const start = f.connection.start();
  const socket = f.sockets.at(-1)!;
  socket.emit('open', new Event('open'));
  socket.message({ protocolVersion: 1, kind: 'subscribed' });
  await start;
  return socket;
}

afterEach(() => vi.useRealTimers());

it('translates only the transport and keeps event names, argument order, and discriminators intact', async () => {
  const f = fixture();
  const socket = await connect(f);
  expect(JSON.parse(socket.send.mock.calls[0][0])).toEqual({
    protocolVersion: 1,
    kind: 'subscribe',
    clientMode: 'groups',
    accessToken: 'session-token',
  });
  const listener = vi.fn();
  f.connection.on('DeploymentInfoUpdated', listener);
  const deployment = {
    id: 'd1',
    spec: { image: { $type: 'External', imageTag: 'nginx', resolvedDigest: 'sha256:applied' } },
  };
  socket.message({
    protocolVersion: 1,
    kind: 'event',
    target: 'DeploymentInfoUpdated',
    arguments: [deployment, 'update'],
  });
  expect(listener).toHaveBeenCalledWith(deployment, 'update');
  f.connection.off('DeploymentInfoUpdated', listener);
  socket.message({
    protocolVersion: 1,
    kind: 'event',
    target: 'DeploymentInfoUpdated',
    arguments: [deployment, 'delete'],
  });
  expect(listener).toHaveBeenCalledTimes(1);
  await f.connection.stop();
});

it('correlates invocation results and propagates authorization failures', async () => {
  const f = fixture();
  const socket = await connect(f);
  const join = f.connection.invoke('JoinGroup', 'platforms');
  const leave = f.connection.invoke('LeaveGroup', 'platforms');
  const rejected = expect(join).rejects.toThrow('Not authorized');
  const calls = socket.send.mock.calls.slice(1).map(([text]) => JSON.parse(text));
  socket.message({ protocolVersion: 1, kind: 'completion', invocationId: calls[1].invocationId, result: null });
  await expect(leave).resolves.toBeNull();
  socket.message({
    protocolVersion: 1,
    kind: 'completion',
    invocationId: calls[0].invocationId,
    error: 'Not authorized',
  });
  await rejected;
  await f.connection.stop();
});

it('rejects pending invocations and cancels reconnection on disposal', async () => {
  vi.useFakeTimers();
  const f = fixture();
  const socket = await connect(f);
  const pending = expect(f.connection.invoke('JoinGroup', 'platforms')).rejects.toThrow('closed');
  socket.emit('close', new Event('close'));
  await pending;
  expect(f.connection.state).toBe(RealtimeConnectionState.Reconnecting);
  await f.connection.stop();
  await vi.runAllTimersAsync();
  expect(f.sockets).toHaveLength(1);
  expect(f.connection.state).toBe(RealtimeConnectionState.Disconnected);
});

it('reconnects with a fresh token and ignores events from the previous socket', async () => {
  vi.useFakeTimers();
  const f = fixture();
  const first = await connect(f);
  const restored = vi.fn(),
    handler = vi.fn();
  f.connection.onreconnected(restored);
  f.connection.on('PlatformUpdated', handler);
  first.emit('close', new Event('close'));
  f.accessTokenFactory.mockReturnValue('refreshed-token');
  await vi.advanceTimersByTimeAsync(0);
  const next = f.sockets[1];
  next.emit('open', new Event('open'));
  next.message({ protocolVersion: 1, kind: 'subscribed' });
  await Promise.resolve();
  expect(restored).toHaveBeenCalledTimes(1);
  expect(JSON.parse(next.send.mock.calls[0][0]).accessToken).toBe('refreshed-token');
  first.message({ protocolVersion: 1, kind: 'event', target: 'PlatformUpdated', arguments: [{}] });
  expect(handler).not.toHaveBeenCalled();
  await f.connection.stop();
});

it('bounds invocation lifetime and pending requests without a global timeout increase', async () => {
  vi.useFakeTimers();
  const f = fixture();
  await connect(f);
  const calls = Array.from({ length: 128 }, () =>
    expect(f.connection.invoke('JoinGroup', 'platforms')).rejects.toThrow('timed out'),
  );
  await expect(f.connection.invoke('JoinGroup', 'platforms')).rejects.toThrow('Too many pending');
  await vi.advanceTimersByTimeAsync(15_000);
  await Promise.all(calls);
  await f.connection.stop();
  expect(vi.getTimerCount()).toBe(0);
});

it('backs off across short successful handshakes and resets after sustained connectivity', async () => {
  vi.useFakeTimers();
  const f = fixture();
  let socket = await connect(f);
  for (const base of [0, 2000, 5000, 10000, 30000, 30000, 0]) {
    const before = f.sockets.length;
    if (before === 7) await vi.advanceTimersByTimeAsync(30000);
    socket.emit('close', new Event('close'));
    const minimum = Math.max(0, base * 0.75 - 1);
    await vi.advanceTimersByTimeAsync(minimum);
    if (base) expect(f.sockets).toHaveLength(before);
    await vi.advanceTimersByTimeAsync(base - minimum);
    expect(f.sockets).toHaveLength(before + 1);
    socket = f.sockets.at(-1)!;
    socket.emit('open', new Event('open'));
    socket.message({ protocolVersion: 1, kind: 'subscribed' });
  }
  await f.connection.stop();
});

it('isolates scoped events and discards late delivery after leaving and rejoining', async () => {
  const f = fixture();
  const socket = await connect(f);
  const scopes = ['Prod', 'Stage'].map((tag) => collectionGroup('git-repositories', [tag]));
  const handlers = scopes.map(() => vi.fn());
  const unfiltered = vi.fn();
  const event = 'GitRepositoryInfoUpdated';
  f.connection.on(event, unfiltered);
  const invoke = async (method: string, key: string) => {
    const done = f.connection.invoke(method, key);
    const frame = JSON.parse(socket.send.mock.calls.at(-1)![0]);
    socket.message({ protocolVersion: 1, kind: 'completion', invocationId: frame.invocationId });
    await done;
    return frame.arguments[0];
  };
  const ids: string[] = [];
  for (const [i, key] of scopes.entries()) {
    f.connection.on(collectionEventKey(key, event), handlers[i]);
    const arg = await invoke('JoinGroup', key);
    expect(arg.tags).toEqual([i === 0 ? 'Prod' : 'Stage']);
    ids.push(arg.subscriptionId);
  }
  const emit = (subscriptionId?: string) =>
    socket.message({ protocolVersion: 1, kind: 'event', target: event, arguments: [{}], subscriptionId });
  emit(ids[0]);
  emit(ids[1]);
  emit();
  expect(await invoke('LeaveGroup', scopes[0])).toBe(ids[0]);
  const next = await invoke('JoinGroup', scopes[0]);
  emit(ids[0]);
  emit(next.subscriptionId);
  expect(handlers[0]).toHaveBeenCalledTimes(2);
  expect(handlers[1]).toHaveBeenCalledOnce();
  expect(unfiltered).toHaveBeenCalledOnce();
  await f.connection.stop();
});
