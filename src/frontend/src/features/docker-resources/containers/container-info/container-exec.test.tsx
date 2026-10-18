import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { RealtimeContext } from '@/lib/context/realtime-context';
import { RealtimeConnection, RealtimeConnectionState } from '@/lib/realtime-connection';
import { ContainerExec } from './container-exec';

vi.mock('@/lib/appearance/appearance-context', () => ({ useAppearance: () => ({ effectiveMode: 'light' }) }));
vi.mock('nanoid', () => ({ nanoid: () => 'terminal-session' }));
vi.mock('@xterm/xterm', () => ({
  Terminal: class {
    options = {};
    cols = 80;
    rows = 24;
    loadAddon() {}
    open() {}
    onData() {
      return { dispose() {} };
    }
    dispose() {}
    write() {}
    writeln() {}
    clear() {}
    focus() {}
  },
}));
vi.mock('@xterm/addon-fit', () => ({
  FitAddon: class {
    fit() {}
  },
}));

function setup() {
  const handlers = new Map<string, (...args: unknown[]) => void>();
  const invoke = vi.fn().mockResolvedValue(undefined);
  const connection = {
    state: RealtimeConnectionState.Connected,
    on: (name: string, handler: (...args: unknown[]) => void) => handlers.set(name, handler),
    off: (name: string) => handlers.delete(name),
    invoke,
  } as unknown as RealtimeConnection;
  const joinGroup = vi.fn().mockResolvedValue(undefined);
  const leaveGroup = vi.fn().mockResolvedValue(undefined);
  const result = render(
    <RealtimeContext.Provider
      value={{
        groups: { connection, connectionState: RealtimeConnectionState.Connected, joinGroup, leaveGroup },
        liveConnectionState: 'connected',
        retryConnection: async () => {},
      }}>
      <ContainerExec containerId="eb2554d5-61e8-45b4-85b6-4276813ec6ce" />
    </RealtimeContext.Provider>,
  );
  return { ...result, user: userEvent.setup(), handlers, invoke, leaveGroup, joinGroup };
}

it('leaves the owned terminal group when the terminal view is closed', async () => {
  const f = setup();
  await f.user.click(screen.getByRole('button', { name: 'Connect' }));
  await waitFor(() =>
    expect(f.invoke).toHaveBeenCalledWith('StartExecProcess', expect.any(String), 'terminal-session', 'bash'),
  );
  const group = f.joinGroup.mock.calls[0][0];
  f.unmount();
  await waitFor(() => expect(f.leaveGroup).toHaveBeenCalledExactlyOnceWith(group));
  expect(f.handlers.has('ContainerExecClosed')).toBe(false);
});

it('leaves the owned terminal group when Disconnect is clicked', async () => {
  const f = setup();
  await f.user.click(screen.getByRole('button', { name: 'Connect' }));
  await f.user.click(await screen.findByRole('button', { name: 'Disconnect' }));
  await waitFor(() => expect(f.leaveGroup).toHaveBeenCalledExactlyOnceWith(f.joinGroup.mock.calls[0][0]));
  expect(screen.getByRole('button', { name: 'Connect' })).toBeVisible();
  expect(f.handlers.has('SendContainerExec')).toBe(false);
});

it('releases an expired terminal and allows reconnecting without affecting another session', async () => {
  const f = setup();
  await f.user.click(screen.getByRole('button', { name: 'Connect' }));
  await screen.findByRole('button', { name: 'Disconnect' });
  act(() => f.handlers.get('ContainerExecClosed')?.('another-terminal'));
  expect(screen.getByRole('button', { name: 'Disconnect' })).toBeVisible();
  expect(f.leaveGroup).not.toHaveBeenCalled();
  act(() => f.handlers.get('ContainerExecClosed')?.(f.joinGroup.mock.calls[0][0]));
  await screen.findByRole('button', { name: 'Connect' });
  await waitFor(() => expect(f.leaveGroup).toHaveBeenCalledTimes(1));
  await f.user.click(screen.getByRole('button', { name: 'Connect' }));
  await waitFor(() => expect(f.invoke.mock.calls.filter(([method]) => method === 'StartExecProcess')).toHaveLength(2));
});
