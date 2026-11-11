import { ApiClientContext } from '@/api/api-client-context';
import { createApiClient } from '@/api/api-client-provider';
import { server } from '@/test/server';
import { PasswordFeedback } from '@/components/custom/password-feedback';
import { RealtimeProvider } from '@/lib/context/realtime-provider';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useSetupContext } from './setup-context';
import { SetupProvider } from './setup-provider';
import { createQueryClient } from '@/query-client-wrapper';
import { QueryClientProvider } from '@tanstack/react-query';

const statusUrl = 'http://localhost/api/v1/setup/status';

function SetupProbe() {
  const setup = useSetupContext();
  return (
    <>
      <span data-testid="policy">
        {setup.passwordMinimumLength}–{setup.passwordMaximumLength}
      </span>
      <span data-testid="setup-status">
        {!setup.isSetupReady ? 'loading' : setup.requiresSetup ? 'pending' : setup.error ? 'error' : 'complete'}
      </span>
      <PasswordFeedback
        password=""
        minimumLength={setup.passwordMinimumLength}
        maximumLength={setup.passwordMaximumLength}
      />
      <button onClick={setup.retry}>Retry startup</button>
      {setup.error && (
        <>
          <span>{setup.error.title}</span>
          <span>{setup.error.message}</span>
        </>
      )}
    </>
  );
}

describe('SetupProvider', () => {
  it('reports a pending first-run setup without polling', async () => {
    let requests = 0;
    server.use(
      http.get(statusUrl, () => {
        requests += 1;
        return HttpResponse.json({ requiresSetup: true, passwordMinimumLength: 8, passwordMaximumLength: 128 });
      }),
    );

    const apiClient = createApiClient('http://localhost');
    render(
      <QueryClientProvider client={createQueryClient()}>
        <ApiClientContext.Provider value={{ apiClient }}>
          <SetupProvider>
            <SetupProbe />
          </SetupProvider>
        </ApiClientContext.Provider>
      </QueryClientProvider>,
    );

    expect(await screen.findByText('pending')).toBeInTheDocument();
    expect(requests).toBe(1);
    expect(screen.getByTestId('policy')).toHaveTextContent('8–128');
  });

  it('reports an unreachable server as a connection problem', async () => {
    server.use(http.get(statusUrl, () => HttpResponse.error()));

    const apiClient = createApiClient('http://localhost');
    render(
      <QueryClientProvider client={createQueryClient()}>
        <ApiClientContext.Provider value={{ apiClient }}>
          <SetupProvider>
            <SetupProbe />
          </SetupProvider>
        </ApiClientContext.Provider>
      </QueryClientProvider>,
    );

    expect(await screen.findByText('Cannot connect to Citadel')).toBeInTheDocument();
    expect(
      screen.getByText('The Citadel server is not reachable. Check that it is running and try again.'),
    ).toBeVisible();
  });

  it('distinguishes an HTTP startup-status failure from a network failure', async () => {
    server.use(http.get(statusUrl, () => new HttpResponse(null, { status: 503 })));

    const apiClient = createApiClient('http://localhost');
    render(
      <QueryClientProvider client={createQueryClient()}>
        <ApiClientContext.Provider value={{ apiClient }}>
          <SetupProvider>
            <SetupProbe />
          </SetupProvider>
        </ApiClientContext.Provider>
      </QueryClientProvider>,
    );

    expect(await screen.findByText('Citadel unavailable')).toBeInTheDocument();
    expect(
      screen.getByText('Citadel could not load its startup status. Check the server logs and try again.'),
    ).toBeVisible();
  });
  it('refreshes password guidance when Core reconnects without resetting the startup gate', async () => {
    let minimumLength = 15;
    let requests = 0;
    server.use(
      http.get(statusUrl, () => {
        requests += 1;
        return HttpResponse.json({
          requiresSetup: false,
          passwordMinimumLength: minimumLength,
          passwordMaximumLength: 128,
        });
      }),
    );
    const connection = new FakeRealtimeConnection();
    const { queryClient } = renderCitadel(
      <SetupProvider>
        <RealtimeProvider
          realtimeTransport="WebSocketV1"
          connectionFactory={() => connection.asRealtimeConnection()}
          startConnection={(conn) => conn.start()}>
          <SetupProbe />
        </RealtimeProvider>
      </SetupProvider>,
    );
    expect(await screen.findByText('Use 15 to 128 characters.')).toBeVisible();
    await waitFor(() => expect(queryClient.isFetching()).toBe(0));
    const initialRequests = requests;
    act(() => connection.reconnecting());
    minimumLength = 10;
    expect(screen.getByTestId('setup-status')).toHaveTextContent('complete');
    expect(requests).toBe(initialRequests);
    act(() => connection.reconnected());
    expect(await screen.findByText('Use 10 to 128 characters.')).toBeVisible();
    expect(screen.getByTestId('setup-status')).toHaveTextContent('complete');
    expect(requests).toBe(initialRequests + 1);
  });

  it('keeps the running policy during a failed background refresh and recovers on the next refresh', async () => {
    server.use(
      http.get(statusUrl, () =>
        HttpResponse.json({ requiresSetup: false, passwordMinimumLength: 15, passwordMaximumLength: 128 }),
      ),
    );
    const { queryClient } = renderCitadel(
      <SetupProvider>
        <SetupProbe />
      </SetupProvider>,
    );
    expect(await screen.findByText('complete')).toBeVisible();
    server.use(http.get(statusUrl, () => HttpResponse.error()));
    await act(async () => {
      await queryClient.invalidateQueries({ queryKey: ['getSetupStatus'] });
    });
    expect(screen.getByTestId('setup-status')).toHaveTextContent('complete');
    expect(screen.getByText('Use 15 to 128 characters.')).toBeVisible();
    server.use(
      http.get(statusUrl, () =>
        HttpResponse.json({ requiresSetup: false, passwordMinimumLength: 10, passwordMaximumLength: 128 }),
      ),
    );
    await act(async () => {
      await queryClient.invalidateQueries({ queryKey: ['getSetupStatus'] });
    });
    expect(await screen.findByText('Use 10 to 128 characters.')).toBeVisible();
  });

  it('allows retrying a failed startup read', async () => {
    server.use(http.get(statusUrl, () => HttpResponse.error()));
    renderCitadel(
      <SetupProvider>
        <SetupProbe />
      </SetupProvider>,
    );
    expect(await screen.findByText('Cannot connect to Citadel')).toBeVisible();
    server.use(
      http.get(statusUrl, () =>
        HttpResponse.json({ requiresSetup: false, passwordMinimumLength: 10, passwordMaximumLength: 128 }),
      ),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Retry startup' }));
    expect(await screen.findByText('complete')).toBeVisible();
    expect(screen.getByText('Use 10 to 128 characters.')).toBeVisible();
  });
});
