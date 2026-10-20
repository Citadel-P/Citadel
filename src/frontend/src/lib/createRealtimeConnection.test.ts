import { createRealtimeConnection, isRealtimeTransportEnabled } from './createRealtimeConnection';
import { FakeWebSocket } from '@/test/fakes/websocket';

beforeEach(() => vi.clearAllMocks());

it('uses the native WebSocket adapter for the supported capability', async () => {
  const socket = new FakeWebSocket();
  const socketFactory = vi.fn(() => socket.asWebSocket());
  const connection = createRealtimeConnection(
    'WebSocketV1',
    { baseUrl: 'http://localhost:8000', accessTokenFactory: () => 'token' },
    socketFactory,
  );
  const started = connection.start();
  socket.emit('open', new Event('open'));
  socket.message({ protocolVersion: 1, kind: 'subscribed' });
  await started;
  expect(socketFactory).toHaveBeenCalledWith('ws://localhost:8000/api/v1/realtime');
  await connection.stop();
});

it.each([undefined, 'None', 'unknown'])(
  'does not connect for unresolved, disabled or unsupported transport %s',
  (transport) => {
    const socketFactory = vi.fn();
    expect(isRealtimeTransportEnabled(transport)).toBe(false);
    expect(() =>
      createRealtimeConnection(transport, { baseUrl: '', accessTokenFactory: () => '' }, socketFactory),
    ).toThrow('Realtime transport is not enabled');
    expect(socketFactory).not.toHaveBeenCalled();
  },
);
