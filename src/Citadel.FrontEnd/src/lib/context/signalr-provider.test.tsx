import { act, render, screen, waitFor } from '@testing-library/react';
import { PropsWithChildren, useCallback, useState } from 'react';
import { AuthContext, AuthContextValue } from '@/features/auth/auth-context';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { FakeHubConnection } from '@/test/fakes/signalr';
import { SignalRProvider } from './signalr-provider';

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

function SignalRTestRoot({
  children,
  fake,
}: PropsWithChildren<{ fake: FakeHubConnection }>) {
  return (
    <AuthContext.Provider value={authValue}>
      <SignalRProvider
        connectionFactory={() => fake.asHubConnection()}
        startConnection={(connection) => connection.start()}
      >
        {children}
      </SignalRProvider>
    </AuthContext.Provider>
  );
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
    expect(fake.send).toHaveBeenCalledTimes(1);
    expect(fake.send).toHaveBeenLastCalledWith('JoinGroup', 'Stacks:1');

    await user.click(screen.getByRole('button', { name: 'remove second' }));
    expect(fake.send).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole('button', { name: 'remove first' }));
    await waitFor(() => {
      expect(fake.send).toHaveBeenLastCalledWith('LeaveGroup', 'Stacks:1');
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
      const joins = fake.send.mock.calls.filter(
        ([method, group]) => method === 'JoinGroup' && group === 'Builds:2',
      );
      expect(joins).toHaveLength(2);
    });
  });

  it('does not report a failed group join as connected', async () => {
    const fake = new FakeHubConnection();
    fake.send.mockRejectedValueOnce(new Error('join failed'));
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});

    render(
      <SignalRTestRoot fake={fake}>
        <GroupSubscriber name="Alerts" />
      </SignalRTestRoot>,
    );

    await waitFor(() => {
      expect(fake.send).toHaveBeenCalledWith('JoinGroup', 'Alerts');
    });
    expect(screen.getByText('disconnected')).toBeInTheDocument();
    expect(consoleError).toHaveBeenCalledWith('JoinGroup failed:', expect.any(Error));
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
