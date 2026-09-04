import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { HubConnection, HubConnectionState } from '@microsoft/signalr';
import { ComponentProps, PropsWithChildren, StrictMode, useCallback, useEffect, useState } from 'react';
import { AuthContext, AuthContextValue } from '@/features/auth/auth-context';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { FakeHubConnection } from '@/test/fakes/signalr';
import { RealtimeProvider } from './realtime-provider';
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query';
import { RealtimeContextType, useRealtimeContext } from './realtime-context';

const authValue: AuthContextValue = {
  accessToken: 'access-token',
  isAuthenticated: true,
  isAuthReady: true,
  isPending: false,
  validationErrors: undefined,
  logout: () => {},
  login: async () => undefined,
  completeLogin: () => {},
};

function GroupSubscriber({ name }: { name: string }) {
  const setupEventListeners = useCallback(() => {}, []);
  const removeEventListeners = useCallback(() => {}, []);
  const { isConnected } = useSignalRGroup({
    groupName: name,
    setupEventListeners,
    removeEventListeners,
  });

  return <span>{isConnected ? 'connected' : 'disconnected'}</span>;
}

function EventSubscriber({ name }: { name: string }) {
  const [message, setMessage] = useState('waiting');
  const handleEvent = useCallback((value: string) => setMessage(value), []);
  const setupEventListeners = useCallback((hub: HubConnection) => hub.on('StreamEvent', handleEvent), [handleEvent]);
  const removeEventListeners = useCallback((hub: HubConnection) => hub.off('StreamEvent', handleEvent), [handleEvent]);
  const { isConnected } = useSignalRGroup({
    groupName: name,
    setupEventListeners,
    removeEventListeners,
  });

  return (
    <>
      <span>{isConnected ? 'connected' : 'disconnected'}</span>
      <span>{message}</span>
    </>
  );
}

function MultiGroupEventSubscriber({ names }: { names: string[] }) {
  const [message, setMessage] = useState('waiting');
  const handleEvent = useCallback((value: string) => setMessage(value), []);
  const setupEventListeners = useCallback((hub: HubConnection) => hub.on('StreamEvent', handleEvent), [handleEvent]);
  const removeEventListeners = useCallback((hub: HubConnection) => hub.off('StreamEvent', handleEvent), [handleEvent]);
  const { isConnected } = useSignalRGroup({
    groupName: names,
    setupEventListeners,
    removeEventListeners,
  });

  return (
    <>
      <span>{isConnected ? 'connected' : 'disconnected'}</span>
      <span>{message}</span>
    </>
  );
}

function SignalRTestRoot({
  children,
  fake,
  queryClient,
  connectionFactory,
  startConnection,
  realtimeTransport,
  webSocketFactory,
}: PropsWithChildren<{
  fake: FakeHubConnection;
  queryClient?: QueryClient;
  connectionFactory?: ComponentProps<typeof RealtimeProvider>['connectionFactory'];
  startConnection?: ComponentProps<typeof RealtimeProvider>['startConnection'];
  realtimeTransport?: ComponentProps<typeof RealtimeProvider>['realtimeTransport'];
  webSocketFactory?: ComponentProps<typeof RealtimeProvider>['webSocketFactory'];
}>) {
  const [defaultQueryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: { retry: false },
          mutations: { retry: false },
        },
      }),
  );

  return (
    <QueryClientProvider client={queryClient ?? defaultQueryClient}>
      <AuthContext.Provider value={authValue}>
        <RealtimeProvider
          connectionFactory={connectionFactory ?? (() => fake.asHubConnection())}
          startConnection={startConnection ?? ((connection) => connection.start())}
          realtimeTransport={realtimeTransport ?? 'SignalR'}
          webSocketFactory={webSocketFactory}>
          {children}
        </RealtimeProvider>
      </AuthContext.Provider>
    </QueryClientProvider>
  );
}

function LiveQuerySubscriber({
  queryFn,
  queryKey = 'getContainerStats',
}: {
  queryFn: () => Promise<string>;
  queryKey?: string;
}) {
  const { data } = useQuery({
    queryKey: [queryKey, { id: 'resource-1' }],
    queryFn,
    staleTime: Infinity,
  });

  return <span>{data ?? 'loading'}</span>;
}

function ConnectionStatusProbe({ onContext }: { onContext?: (context: RealtimeContextType) => void }) {
  const context = useRealtimeContext();

  useEffect(() => {
    onContext?.(context);
  }, [context, onContext]);

  return (
    <>
      <span data-testid="live-state">{context.liveConnectionState}</span>
      <span data-testid="interrupted-at">{context.interruptedAt ?? 'none'}</span>
    </>
  );
}

class FakeWebSocket {
  private readonly listeners = new Map<string, Set<EventListener>>();

  readonly send = vi.fn();
  readonly close = vi.fn();
  readonly addEventListener = vi.fn((type: string, listener: EventListener) => {
    const listeners = this.listeners.get(type) ?? new Set<EventListener>();
    listeners.add(listener);
    this.listeners.set(type, listeners);
  });

  emit(type: string, event: Event) {
    this.listeners.get(type)?.forEach((listener) => listener.call(this, event));
  }

  asWebSocket(): WebSocket {
    return this as unknown as WebSocket;
  }
}

describe('RealtimeProvider', () => {
  beforeEach(() => {
    Object.defineProperty(window.navigator, 'onLine', {
      configurable: true,
      value: true,
    });
  });

  it('reports the initial connection and established reconnect lifecycle', async () => {
    const fake = new FakeHubConnection();
    const startGate = deferred<void>();
    fake.start.mockImplementation(async () => {
      await startGate.promise;
      fake.state = HubConnectionState.Connected;
    });

    render(
      <SignalRTestRoot fake={fake}>
        <ConnectionStatusProbe />
      </SignalRTestRoot>,
    );

    expect(screen.getByTestId('live-state')).toHaveTextContent('connecting');
    startGate.resolve();
    await waitFor(() => expect(screen.getByTestId('live-state')).toHaveTextContent('connected'));

    act(() => {
      fake.reconnecting();
    });
    expect(screen.getByTestId('live-state')).toHaveTextContent('reconnecting');
    expect(screen.getByTestId('interrupted-at')).not.toHaveTextContent('none');

    act(() => {
      fake.closed();
    });
    expect(screen.getByTestId('live-state')).toHaveTextContent('disconnected');
  });

  it('distinguishes browser offline and waits for SignalR before reporting recovery', async () => {
    const fake = new FakeHubConnection();
    render(
      <SignalRTestRoot fake={fake}>
        <ConnectionStatusProbe />
      </SignalRTestRoot>,
    );

    await waitFor(() => expect(screen.getByTestId('live-state')).toHaveTextContent('connected'));

    Object.defineProperty(window.navigator, 'onLine', {
      configurable: true,
      value: false,
    });
    act(() => {
      window.dispatchEvent(new Event('offline'));
      fake.reconnecting();
    });
    expect(screen.getByTestId('live-state')).toHaveTextContent('offline');

    Object.defineProperty(window.navigator, 'onLine', {
      configurable: true,
      value: true,
    });
    act(() => {
      window.dispatchEvent(new Event('online'));
    });
    expect(screen.getByTestId('live-state')).toHaveTextContent('reconnecting');

    act(() => {
      fake.reconnected();
    });
    await waitFor(() => expect(screen.getByTestId('live-state')).toHaveTextContent('connected'));
  });

  it('uses one fresh connection for concurrent manual retries', async () => {
    const first = new FakeHubConnection();
    const second = new FakeHubConnection();
    const retryGate = deferred<void>();
    second.start.mockImplementation(async () => {
      await retryGate.promise;
      second.state = HubConnectionState.Connected;
    });

    const factory = vi.fn().mockReturnValueOnce(first.asHubConnection()).mockReturnValueOnce(second.asHubConnection());
    let context: RealtimeContextType | undefined;

    render(
      <SignalRTestRoot fake={first} connectionFactory={factory} queryClient={new QueryClient()}>
        <ConnectionStatusProbe onContext={(value) => (context = value)} />
      </SignalRTestRoot>,
    );

    await waitFor(() => expect(screen.getByTestId('live-state')).toHaveTextContent('connected'));
    act(() => {
      first.closed();
    });
    expect(screen.getByTestId('live-state')).toHaveTextContent('disconnected');

    let firstRetry!: Promise<void>;
    let secondRetry!: Promise<void>;
    act(() => {
      firstRetry = context!.retryConnection();
      secondRetry = context!.retryConnection();
    });

    expect(firstRetry).toBe(secondRetry);
    await waitFor(() => expect(second.start).toHaveBeenCalledOnce());
    expect(factory).toHaveBeenCalledTimes(2);

    retryGate.resolve();
    await act(async () => {
      await firstRetry;
    });
    expect(screen.getByTestId('live-state')).toHaveTextContent('connected');
  });

  it('keeps the disconnected state when a manual retry fails', async () => {
    const first = new FakeHubConnection();
    const second = new FakeHubConnection();
    second.start.mockRejectedValue(new Error('Core unavailable'));

    const factory = vi.fn().mockReturnValueOnce(first.asHubConnection()).mockReturnValueOnce(second.asHubConnection());
    let context: RealtimeContextType | undefined;

    render(
      <SignalRTestRoot fake={first} connectionFactory={factory}>
        <ConnectionStatusProbe onContext={(value) => (context = value)} />
      </SignalRTestRoot>,
    );

    await waitFor(() => expect(screen.getByTestId('live-state')).toHaveTextContent('connected'));
    act(() => {
      first.closed();
    });

    await act(async () => {
      await expect(context!.retryConnection()).rejects.toThrow('Core unavailable');
    });

    expect(second.start).toHaveBeenCalledOnce();
    expect(screen.getByTestId('live-state')).toHaveTextContent('disconnected');
  });

  it('does not publish an outage while replacing a connection after token rotation', async () => {
    const first = new FakeHubConnection();
    const second = new FakeHubConnection();
    const factory = vi.fn().mockReturnValueOnce(first.asHubConnection()).mockReturnValueOnce(second.asHubConnection());
    const observedStates: string[] = [];

    function TokenRotationRoot() {
      const [token, setToken] = useState('token-1');
      return (
        <QueryClientProvider client={new QueryClient()}>
          <AuthContext.Provider value={{ ...authValue, accessToken: token }}>
            <RealtimeProvider
              connectionFactory={factory}
              startConnection={(candidate) => candidate.start()}
              realtimeTransport="SignalR">
              <ConnectionStatusProbe onContext={(value) => observedStates.push(value.liveConnectionState)} />
              <button onClick={() => setToken('token-2')}>rotate token</button>
            </RealtimeProvider>
          </AuthContext.Provider>
        </QueryClientProvider>
      );
    }

    render(<TokenRotationRoot />);
    await waitFor(() => expect(screen.getByTestId('live-state')).toHaveTextContent('connected'));
    observedStates.length = 0;

    fireEvent.click(screen.getByRole('button', { name: 'rotate token' }));
    await waitFor(() => expect(second.start).toHaveBeenCalledOnce());
    await waitFor(() => expect(screen.getByTestId('live-state')).toHaveTextContent('connected'));

    expect(observedStates).not.toContain('disconnected');
    expect(observedStates).not.toContain('reconnecting');
    expect(observedStates).not.toContain('offline');
  });

  it('removes browser lifecycle listeners when disposed', async () => {
    const fake = new FakeHubConnection();
    const addListener = vi.spyOn(window, 'addEventListener');
    const removeListener = vi.spyOn(window, 'removeEventListener');
    const view = render(
      <SignalRTestRoot fake={fake}>
        <ConnectionStatusProbe />
      </SignalRTestRoot>,
    );

    await waitFor(() => expect(screen.getByTestId('live-state')).toHaveTextContent('connected'));
    const offlineHandler = addListener.mock.calls.find(([eventName]) => eventName === 'offline')?.[1];
    const onlineHandler = addListener.mock.calls.find(([eventName]) => eventName === 'online')?.[1];

    view.unmount();

    expect(removeListener).toHaveBeenCalledWith('offline', offlineHandler);
    expect(removeListener).toHaveBeenCalledWith('online', onlineHandler);
  });

  it('joins a shared group once and leaves it after the last subscriber unmounts', async () => {
    const fake = new FakeHubConnection();

    function Subscribers() {
      const [secondVisible, setSecondVisible] = useState(true);
      const [firstVisible, setFirstVisible] = useState(true);

      return (
        <>
          {firstVisible && <GroupSubscriber name="Stacks:1" />}
          {secondVisible && <GroupSubscriber name="Stacks:1" />}
          <button onClick={() => setSecondVisible(false)}>remove second</button>
          <button onClick={() => setFirstVisible(false)}>remove first</button>
        </>
      );
    }

    const { user } = setupUser();
    render(
      <SignalRTestRoot fake={fake}>
        <Subscribers />
      </SignalRTestRoot>,
    );

    await waitFor(() => {
      expect(screen.getAllByText('connected')).toHaveLength(2);
    });
    expect(fake.invoke).toHaveBeenCalledTimes(1);
    expect(fake.invoke).toHaveBeenLastCalledWith('JoinGroup', 'Stacks:1');

    await user.click(screen.getByRole('button', { name: 'remove second' }));
    expect(fake.invoke).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole('button', { name: 'remove first' }));
    await waitFor(() => {
      expect(fake.invoke).toHaveBeenLastCalledWith('LeaveGroup', 'Stacks:1');
    });
  });

  it('rejoins active groups after a reconnect', async () => {
    const fake = new FakeHubConnection();
    render(
      <SignalRTestRoot fake={fake}>
        <GroupSubscriber name="Builds:2" />
      </SignalRTestRoot>,
    );

    await screen.findByText('connected');

    act(() => {
      fake.reconnecting();
    });
    await screen.findByText('disconnected');

    act(() => {
      fake.reconnected();
    });

    await screen.findByText('connected');
    await waitFor(() => {
      const joins = fake.invoke.mock.calls.filter(([method, group]) => method === 'JoinGroup' && group === 'Builds:2');
      expect(joins).toHaveLength(2);
    });
  });

  it('retains an active group when one rejoin attempt fails', async () => {
    const fake = new FakeHubConnection();
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});

    render(
      <SignalRTestRoot fake={fake}>
        <GroupSubscriber name="Builds:retry" />
      </SignalRTestRoot>,
    );

    await screen.findByText('connected');
    fake.invoke.mockRejectedValueOnce(new Error('temporary rejoin failure'));

    act(() => {
      fake.reconnecting();
      fake.reconnected();
    });

    await waitFor(() => {
      const joins = fake.invoke.mock.calls.filter(
        ([method, group]) => method === 'JoinGroup' && group === 'Builds:retry',
      );
      expect(joins).toHaveLength(2);
    });

    act(() => {
      fake.reconnecting();
      fake.reconnected();
    });

    await waitFor(() => {
      const joins = fake.invoke.mock.calls.filter(
        ([method, group]) => method === 'JoinGroup' && group === 'Builds:retry',
      );
      expect(joins).toHaveLength(3);
    });
    consoleError.mockRestore();
  });

  it('refreshes an active live-backed query once after reconnecting', async () => {
    const fake = new FakeHubConnection();
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });
    const queryFn = vi.fn().mockResolvedValueOnce('initial').mockResolvedValueOnce('caught-up');

    render(
      <SignalRTestRoot fake={fake} queryClient={queryClient}>
        <LiveQuerySubscriber queryFn={queryFn} queryKey="listStacks" />
      </SignalRTestRoot>,
    );

    expect(await screen.findByText('initial')).toBeInTheDocument();
    expect(queryFn).toHaveBeenCalledTimes(1);

    act(() => {
      fake.reconnecting();
      fake.reconnected();
    });

    expect(await screen.findByText('caught-up')).toBeInTheDocument();
    expect(queryFn).toHaveBeenCalledTimes(2);
  });

  it('does not refetch an inactive live-backed query after reconnecting', async () => {
    const fake = new FakeHubConnection();
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });
    const queryKey = ['listStacks', { query: { tags: ['Prod'] } }];
    const queryFn = vi.fn().mockResolvedValue('cached');
    await queryClient.fetchQuery({ queryKey, queryFn, staleTime: Infinity });

    render(
      <SignalRTestRoot fake={fake} queryClient={queryClient}>
        <ConnectionStatusProbe />
      </SignalRTestRoot>,
    );
    await waitFor(() => expect(screen.getByTestId('live-state')).toHaveTextContent('connected'));

    act(() => {
      fake.reconnecting();
      fake.reconnected();
    });

    await waitFor(() => expect(queryClient.getQueryState(queryKey)?.isInvalidated).toBe(true));
    expect(queryFn).toHaveBeenCalledOnce();
  });

  it('does not report a failed group join as connected', async () => {
    const fake = new FakeHubConnection();
    fake.invoke.mockRejectedValueOnce(new Error('join failed'));
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});

    render(
      <SignalRTestRoot fake={fake}>
        <GroupSubscriber name="Alerts" />
      </SignalRTestRoot>,
    );

    await waitFor(() => {
      expect(fake.invoke).toHaveBeenCalledWith('JoinGroup', 'Alerts');
    });
    expect(screen.getByText('disconnected')).toBeInTheDocument();
    expect(consoleError).toHaveBeenCalledWith('JoinGroup failed:', expect.any(Error));
  });

  it('preserves event listeners during StrictMode effect replay', async () => {
    const fake = new FakeHubConnection();

    render(
      <SignalRTestRoot fake={fake}>
        <StrictMode>
          <EventSubscriber name="container-log:0123456789ab" />
        </StrictMode>
      </SignalRTestRoot>,
    );

    await screen.findByText('connected');
    expect(fake.listenerCount('StreamEvent')).toBe(1);

    act(() => {
      fake.emit('StreamEvent', 'streamed');
    });

    expect(await screen.findByText('streamed')).toBeInTheDocument();
  });

  it('reports the Rust WebSocket lifecycle and refreshes license state without reading event payloads', async () => {
    const fake = new FakeHubConnection();
    fake.start.mockResolvedValue();
    const socket = new FakeWebSocket();
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const invalidate = vi.spyOn(queryClient, 'invalidateQueries');
    const rendered = render(
      <SignalRTestRoot
        fake={fake}
        queryClient={queryClient}
        realtimeTransport="WebSocketV1"
        webSocketFactory={() => socket.asWebSocket()}>
        <ConnectionStatusProbe />
      </SignalRTestRoot>,
    );

    expect(screen.getByTestId('live-state')).toHaveTextContent('connecting');
    act(() => socket.emit('open', new Event('open')));
    expect(fake.start).not.toHaveBeenCalled();
    expect(socket.send).toHaveBeenCalledWith(
      JSON.stringify({ protocolVersion: 1, kind: 'subscribe', accessToken: 'access-token' }),
    );
    act(() =>
      socket.emit(
        'message',
        new MessageEvent('message', {
          data: JSON.stringify({ protocolVersion: 1, kind: 'subscribed' }),
        }),
      ),
    );
    await waitFor(() => expect(screen.getByTestId('live-state')).toHaveTextContent('connected'));
    await waitFor(() =>
      expect(invalidate).toHaveBeenCalledWith({ queryKey: ['getLicenseEntitlements'] }),
    );
    invalidate.mockClear();

    act(() =>
      socket.emit(
        'message',
        new MessageEvent('message', {
          data: JSON.stringify({
            protocolVersion: 1,
            eventKind: 'licenseStateChanged',
            payload: { ignored: 'must-not-be-used' },
          }),
        }),
      ),
    );
    await waitFor(() => {
      expect(invalidate).toHaveBeenCalledWith({ queryKey: ['getLicenseEntitlements'] });
      expect(invalidate).toHaveBeenCalledWith({ queryKey: ['getLicense'] });
    });
    invalidate.mockClear();

    act(() =>
      socket.emit(
        'message',
        new MessageEvent('message', {
          data: JSON.stringify({
            protocolVersion: 1,
            resourceType: 'Registry',
            eventKind: 'registryChanged',
          }),
        }),
      ),
    );
    await waitFor(() =>
      expect(invalidate).toHaveBeenCalledWith({ predicate: expect.any(Function) }),
    );
    const predicate = invalidate.mock.calls[0]?.[0]?.predicate;
    if (!predicate) throw new Error('Registry invalidation predicate was not registered.');
    type InvalidatedQuery = Parameters<typeof predicate>[0];
    expect(predicate({ queryKey: ['listRegistries'] } as InvalidatedQuery)).toBe(true);
    expect(predicate({ queryKey: ['listPlatforms'] } as InvalidatedQuery)).toBe(false);

    rendered.unmount();
    expect(socket.close).toHaveBeenCalled();
  });

  it('uses one listener while joining and leaving multiple groups', async () => {
    const fake = new FakeHubConnection();
    const names = ['swarm-services:platform-1', 'swarm-services:platform-2'];

    function ToggleSubscriber() {
      const [visible, setVisible] = useState(true);
      return (
        <>
          {visible && <MultiGroupEventSubscriber names={names} />}
          <button onClick={() => setVisible(false)}>remove multi-group subscriber</button>
        </>
      );
    }

    render(
      <SignalRTestRoot fake={fake}>
        <ToggleSubscriber />
      </SignalRTestRoot>,
    );

    await screen.findByText('connected');
    expect(fake.listenerCount('StreamEvent')).toBe(1);
    expect(fake.invoke).toHaveBeenCalledWith('JoinGroup', names[0]);
    expect(fake.invoke).toHaveBeenCalledWith('JoinGroup', names[1]);

    act(() => fake.emit('StreamEvent', 'multi-group-event'));
    expect(await screen.findByText('multi-group-event')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'remove multi-group subscriber' }));
    await waitFor(() => {
      expect(fake.invoke).toHaveBeenCalledWith('LeaveGroup', names[0]);
      expect(fake.invoke).toHaveBeenCalledWith('LeaveGroup', names[1]);
    });
    expect(fake.listenerCount('StreamEvent')).toBe(0);
  });
});

function setupUser() {
  const user = {
    click: async (element: Element) => {
      await act(async () => {
        element.dispatchEvent(new MouseEvent('click', { bubbles: true }));
      });
    },
  };

  return { user };
}

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });

  return { promise, resolve, reject };
}
