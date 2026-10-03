import { createRealtimeConnection, isRealtimeTransportEnabled } from './createRealtimeConnection';
import { createSignalRConnection } from './createSignalRConnection';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { FakeWebSocket } from '@/test/fakes/websocket';

vi.mock('./createSignalRConnection', () => ({ createSignalRConnection: vi.fn() }));

beforeEach(() => vi.clearAllMocks());

it('uses native WebSocket for Rust without constructing a SignalR connection', async () => {
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
  expect(createSignalRConnection).not.toHaveBeenCalled();
  await connection.stop();
});

it('keeps the .NET transport isolated behind explicit negotiation', () => {
  const fake = new FakeRealtimeConnection();
  vi.mocked(createSignalRConnection).mockReturnValue(fake);
  const options = { baseUrl: 'http://localhost:8000', accessTokenFactory: () => 'token' };
  expect(createRealtimeConnection('SignalR', options)).toBe(fake);
  expect(createSignalRConnection).toHaveBeenCalledWith(options);
});

it.each([undefined, 'None', 'unknown'])('does not default unsupported transport %s to SignalR', (transport) => {
  expect(isRealtimeTransportEnabled(transport)).toBe(false);
  expect(() => createRealtimeConnection(transport, { baseUrl: '', accessTokenFactory: () => '' })).toThrow(
    'Realtime transport is not enabled',
  );
  expect(createSignalRConnection).not.toHaveBeenCalled();
});
