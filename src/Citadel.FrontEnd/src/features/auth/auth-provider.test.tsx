import { ApiClientContext } from '@/api/api-client-context';
import { createApiClient } from '@/api/api-client-provider';
import { LoginNextStep } from '@/api/generated/api.types';
import { createQueryClient } from '@/query-client-wrapper';
import { server } from '@/test/server';
import { QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { useAuthContext } from './auth-context';
import { AuthProvider } from './auth-provider';
import { SetupContext, SetupContextValue } from '@/features/setup/setup-context';

const refreshUrl = 'http://localhost/api/v1/authentication/refresh';
const loginUrl = 'http://localhost/api/v1/authentication/login';
const logoutUrl = 'http://localhost/api/v1/authentication/logout';

const createValidToken = (subject: string) => {
  const encode = (value: object) =>
    btoa(JSON.stringify(value)).replaceAll('+', '-').replaceAll('/', '_').replaceAll('=', '');

  return `${encode({ alg: 'none', typ: 'JWT' })}.${encode({
    exp: Math.floor(Date.now() / 1000) + 300,
    sub: subject,
  })}.signature`;
};

function AuthProbe() {
  const auth = useAuthContext();

  return (
    <>
      <span data-testid="auth-status">
        {auth.isAuthReady ? (auth.isAuthenticated ? 'authenticated' : 'unauthenticated') : 'loading'}
      </span>
      <span data-testid="access-token">{auth.accessToken ?? 'none'}</span>
      <button
        onClick={() =>
          void auth.login({
            emailOrName: 'admin@example.com',
            password: 'password',
          })
        }
      >
        login
      </button>
      <button onClick={auth.logout}>logout</button>
    </>
  );
}

const completeSetup: SetupContextValue = {
  passwordMinimumLength: 15,
  passwordMaximumLength: 128,
  isSetupReady: true,
  requiresSetup: false,
  markSetupComplete: () => {},
  retry: () => {},
};

const renderAuthProvider = (setup: SetupContextValue = completeSetup) => {
  const queryClient = createQueryClient();
  const apiClient = createApiClient('http://localhost');
  const user = userEvent.setup();

  render(
    <QueryClientProvider client={queryClient}>
      <ApiClientContext.Provider value={{ apiClient }}>
        <SetupContext.Provider value={setup}>
          <AuthProvider>
            <AuthProbe />
          </AuthProvider>
        </SetupContext.Provider>
      </ApiClientContext.Provider>
    </QueryClientProvider>,
  );

  return { apiClient, queryClient, user };
};

describe('AuthProvider', () => {
  it('does not refresh a session while initial setup is pending', async () => {
    let refreshRequests = 0;
    server.use(
      http.get(refreshUrl, () => {
        refreshRequests += 1;
        return HttpResponse.json({ accessToken: createValidToken('unexpected') });
      }),
    );

    renderAuthProvider({ ...completeSetup, requiresSetup: true });

    await waitFor(() => {
      expect(screen.getByTestId('auth-status')).toHaveTextContent('unauthenticated');
    });
    expect(refreshRequests).toBe(0);
  });

  it('restores the authenticated session through the refresh cookie on page bootstrap', async () => {
    let refreshRequests = 0;
    const restoredToken = createValidToken('restored');
    server.use(
      http.get(refreshUrl, () => {
        refreshRequests += 1;
        return HttpResponse.json({ accessToken: restoredToken });
      }),
    );

    renderAuthProvider();

    await waitFor(() => {
      expect(screen.getByTestId('auth-status')).toHaveTextContent('authenticated');
    });
    expect(screen.getByTestId('access-token')).toHaveTextContent(restoredToken);
    expect(refreshRequests).toBe(1);
  });

  it('finishes bootstrap unauthenticated after an invalid refresh cookie', async () => {
    let refreshRequests = 0;
    server.use(
      http.get(refreshUrl, () => {
        refreshRequests += 1;
        return HttpResponse.json(
          { status: 401, title: 'Unauthorized', detail: 'Invalid refresh token.' },
          { status: 401 },
        );
      }),
      http.post(logoutUrl, () => new HttpResponse(null, { status: 204 })),
    );
    vi.spyOn(console, 'error').mockImplementation(() => {});

    renderAuthProvider();

    await waitFor(() => {
      expect(screen.getByTestId('auth-status')).toHaveTextContent('unauthenticated');
    });
    expect(screen.getByTestId('access-token')).toHaveTextContent('none');
    expect(refreshRequests).toBe(1);
  });

  it('can log in after an unauthenticated bootstrap', async () => {
    let loginRequest: unknown;
    const loginToken = createValidToken('login');
    server.use(
      http.get(refreshUrl, () =>
        HttpResponse.json({ status: 401, title: 'Unauthorized' }, { status: 401 }),
      ),
      http.post(logoutUrl, () => new HttpResponse(null, { status: 204 })),
      http.post(loginUrl, async ({ request }) => {
        loginRequest = await request.json();
        return HttpResponse.json({
          accessToken: loginToken,
          nextStep: LoginNextStep.Completed,
        });
      }),
    );
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const { user } = renderAuthProvider();

    await screen.findByText('unauthenticated');
    await user.click(screen.getByRole('button', { name: 'login' }));

    await waitFor(() => {
      expect(screen.getByTestId('auth-status')).toHaveTextContent('authenticated');
    });
    expect(loginRequest).toEqual({
      emailOrName: 'admin@example.com',
      password: 'password',
    });
    expect(screen.getByTestId('access-token')).toHaveTextContent(loginToken);
  });

  it('clears the local session and calls the logout endpoint', async () => {
    let logoutRequests = 0;
    const initialToken = createValidToken('logout');
    server.use(
      http.get(refreshUrl, () => HttpResponse.json({ accessToken: initialToken })),
      http.post(logoutUrl, () => {
        logoutRequests += 1;
        return new HttpResponse(null, { status: 204 });
      }),
    );
    const { user } = renderAuthProvider();

    await screen.findByText('authenticated');
    await user.click(screen.getByRole('button', { name: 'logout' }));

    expect(screen.getByTestId('auth-status')).toHaveTextContent('unauthenticated');
    await waitFor(() => {
      expect(logoutRequests).toBe(1);
    });
  });
});
