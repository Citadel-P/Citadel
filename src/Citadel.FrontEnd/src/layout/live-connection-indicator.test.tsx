import { act, fireEvent, render, screen } from '@testing-library/react';
import { HubConnectionState } from '@microsoft/signalr';
import { SignalRContext, SignalRContextType } from '@/lib/context/signalr-context';
import { LiveConnectionIndicator } from './live-connection-indicator';

const { toastSuccess } = vi.hoisted(() => ({
  toastSuccess: vi.fn(),
}));

vi.mock('sonner', () => ({
  toast: {
    success: toastSuccess,
  },
}));

const baseContext: SignalRContextType = {
  connection: null,
  connectionState: HubConnectionState.Disconnected,
  liveConnectionState: 'connected',
  retryConnection: vi.fn(async () => {}),
  joinGroup: vi.fn(async () => {}),
  leaveGroup: vi.fn(async () => {}),
};

const indicator = (overrides: Partial<SignalRContextType> = {}) => (
  <SignalRContext.Provider value={{ ...baseContext, ...overrides }}>
    <LiveConnectionIndicator />
  </SignalRContext.Provider>
);

describe('LiveConnectionIndicator', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('does not reserve UI while connecting normally or connected', () => {
    const view = render(indicator({ liveConnectionState: 'connecting' }));
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();

    view.rerender(indicator({ liveConnectionState: 'connected' }));
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(toastSuccess).not.toHaveBeenCalled();
  });

  it('suppresses fast reconnects without showing a recovery toast', () => {
    const view = render(indicator({ liveConnectionState: 'reconnecting', interruptedAt: 100 }));

    act(() => {
      vi.advanceTimersByTime(1_499);
    });
    expect(screen.queryByRole('status')).not.toBeInTheDocument();

    view.rerender(indicator({ liveConnectionState: 'connected' }));
    act(() => {
      vi.runOnlyPendingTimers();
    });
    expect(toastSuccess).not.toHaveBeenCalled();
  });

  it('shows a delayed reconnecting status and one recovery toast', () => {
    const view = render(indicator({ liveConnectionState: 'reconnecting', interruptedAt: 200 }));

    act(() => {
      vi.advanceTimersByTime(1_500);
    });
    expect(screen.getByRole('status', { name: /Live updates interrupted/ })).toBeVisible();

    view.rerender(indicator({ liveConnectionState: 'connected' }));
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(toastSuccess).toHaveBeenCalledOnce();
    expect(toastSuccess).toHaveBeenCalledWith('Live updates restored');

    view.rerender(indicator({ liveConnectionState: 'connected' }));
    expect(toastSuccess).toHaveBeenCalledOnce();
  });

  it('keeps the interrupted status visible while reconnecting after an offline state', () => {
    const view = render(indicator({ liveConnectionState: 'offline', interruptedAt: 300 }));
    expect(screen.getByRole('status', { name: /You're offline/ })).toBeVisible();

    act(() => {
      vi.runOnlyPendingTimers();
    });
    view.rerender(indicator({ liveConnectionState: 'connecting', interruptedAt: 300 }));

    expect(screen.getByRole('status', { name: /Live updates interrupted/ })).toBeVisible();
  });

  it('offers one guarded retry when automatic reconnect is exhausted', async () => {
    const retryGate = deferred<void>();
    const retryConnection = vi.fn(() => retryGate.promise);
    render(
      indicator({
        liveConnectionState: 'disconnected',
        interruptedAt: 400,
        retryConnection,
      }),
    );

    const retry = screen.getByRole('button', { name: /Retry live updates/ });
    fireEvent.click(retry);
    fireEvent.click(retry);

    expect(retryConnection).toHaveBeenCalledOnce();
    expect(retry).toBeDisabled();
    expect(screen.getByText('Reconnecting...')).toBeInTheDocument();

    await act(async () => {
      retryGate.resolve();
      await retryGate.promise;
    });
    expect(retry).not.toBeDisabled();
  });

  it('shows offline wording without a retry action', () => {
    render(indicator({ liveConnectionState: 'offline', interruptedAt: 500 }));

    expect(screen.getByRole('status', { name: /You're offline/ })).toBeVisible();
    expect(screen.queryByRole('button', { name: /Retry live updates/ })).not.toBeInTheDocument();
  });

  it('cancels the reconnecting grace timer when unmounted', () => {
    const view = render(indicator({ liveConnectionState: 'reconnecting', interruptedAt: 600 }));
    expect(vi.getTimerCount()).toBe(1);

    view.unmount();

    expect(vi.getTimerCount()).toBe(0);
    expect(toastSuccess).not.toHaveBeenCalled();
  });
});

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });

  return { promise, resolve, reject };
}
