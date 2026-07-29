import { ApiClientContext } from '@/api/api-client-context';
import { createApiClient } from '@/api/api-client-provider';
import { server } from '@/test/server';
import { render, screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useSetupContext } from './setup-context';
import { SetupProvider } from './setup-provider';

const statusUrl = 'http://localhost/api/v1/setup/status';

function SetupProbe() {
  const setup = useSetupContext();
  return (
    <>
      <span data-testid="setup-status">
        {!setup.isSetupReady ? 'loading' : setup.requiresSetup ? 'pending' : setup.error ? 'error' : 'complete'}
      </span>
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
        return HttpResponse.json({ requiresSetup: true });
      }),
    );

    const apiClient = createApiClient('http://localhost');
    render(
      <ApiClientContext.Provider value={{ apiClient }}>
        <SetupProvider>
          <SetupProbe />
        </SetupProvider>
      </ApiClientContext.Provider>,
    );

    expect(await screen.findByText('pending')).toBeInTheDocument();
    expect(requests).toBe(1);
  });

  it('reports an unreachable server as a connection problem', async () => {
    server.use(http.get(statusUrl, () => HttpResponse.error()));

    const apiClient = createApiClient('http://localhost');
    render(
      <ApiClientContext.Provider value={{ apiClient }}>
        <SetupProvider>
          <SetupProbe />
        </SetupProvider>
      </ApiClientContext.Provider>,
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
      <ApiClientContext.Provider value={{ apiClient }}>
        <SetupProvider>
          <SetupProbe />
        </SetupProvider>
      </ApiClientContext.Provider>,
    );

    expect(await screen.findByText('Citadel unavailable')).toBeInTheDocument();
    expect(
      screen.getByText('Citadel could not load its startup status. Check the server logs and try again.'),
    ).toBeVisible();
  });
});
