import { act, render, screen, waitFor } from '@testing-library/react';
import { HubConnection } from '@microsoft/signalr';
import { PropsWithChildren, StrictMode, useCallback, useState } from 'react';
import { AuthContext, AuthContextValue } from '@/features/auth/auth-context';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { FakeHubConnection } from '@/test/fakes/signalr';
import { SignalRProvider } from './signalr-provider';
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query';

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

function SignalRTestRoot({
  children,
  fake,
  queryClient,
}: PropsWithChildren<{ fake: FakeHubConnection; queryClient?: QueryClient }>) {
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
        <SignalRProvider
          connectionFactory={() => fake.asHubConnection()}
          startConnection={(connection) => connection.start()}>
          {children}
        </SignalRProvider>
      </AuthContext.Provider>
    </QueryClientProvider>
  );
}

function StatsQuerySubscriber({ queryFn }: { queryFn: () => Promise<string> }) {
  const { data } = useQuery({
    queryKey: ['getContainerStats', { id: 'container-1', query: { hours: 24 } }],
    queryFn,
    staleTime: Infinity,
  });

  return <span>{data ?? 'loading'}</span>;
}

describe('SignalRProvider', () => {
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

  it('refreshes active streamed statistics once after reconnecting', async () => {
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
        <StatsQuerySubscriber queryFn={queryFn} />
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
