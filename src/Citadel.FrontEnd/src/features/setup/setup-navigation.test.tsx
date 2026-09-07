import { ApiClientContext } from '@/api/api-client-context';
import { createApiClient } from '@/api/api-client-provider';
import { LoginNextStep } from '@/api/generated/api.types';
import { AuthProvider } from '@/features/auth/auth-provider';
import { RequireAuth, RequireNoAuth } from '@/features/auth/auth-route-guards';
import { createQueryClient } from '@/query-client-wrapper';
import { server } from '@/test/server';
import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router';
import InitialSetup from '.';
import { SetupProvider } from './setup-provider';
import { RequireSetup, RequireSetupComplete } from './setup-route-guards';

function CurrentPath() {
  const location = useLocation();
  return (
    <span data-testid="path">
      {location.pathname}
      {location.search}
      {location.hash}
    </span>
  );
}

function renderSetup(entry: string) {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <QueryClientProvider client={createQueryClient()}>
        <ApiClientContext.Provider value={{ apiClient: createApiClient('http://localhost') }}>
          <SetupProvider>
            <AuthProvider>
              <CurrentPath />
              <Routes>
                <Route element={<RequireSetup />}>
                  <Route path="/setup" element={<InitialSetup />} />
                </Route>
                <Route element={<RequireSetupComplete />}>
                  <Route element={<RequireNoAuth />}>
                    <Route path="/login" element={<div>Login page</div>} />
                    <Route path="/login/mfa" element={<div>MFA verification</div>} />
                    <Route path="/login/mfa/setup" element={<div>MFA enrollment</div>} />
                  </Route>
                  <Route element={<RequireAuth />}>
                    <Route path="/" element={<div>Platforms page</div>} />
                    <Route path="/stacks" element={<div>Stacks page</div>} />
                  </Route>
                </Route>
              </Routes>
            </AuthProvider>
          </SetupProvider>
        </ApiClientContext.Provider>
      </QueryClientProvider>
    </MemoryRouter>,
  );
}

async function submitAdministrator() {
  await screen.findByRole('button', { name: 'Create administrator' });
  for (const [label, value] of [
    ['Username', 'owner'],
    ['Email address', 'owner@example.test'],
    ['Password', 'correct-horse-battery-staple'],
    ['Confirm password', 'correct-horse-battery-staple'],
  ]) {
    fireEvent.change(screen.getByLabelText(label), { target: { value } });
  }
  fireEvent.click(screen.getByRole('button', { name: 'Create administrator' }));
}

describe('First-run setup navigation', () => {
  it.each([
    ['/login', undefined, '/', 'Platforms page'],
    ['/login?source=start#signin', undefined, '/', 'Platforms page'],
    ['/', undefined, '/', 'Platforms page'],
    ['/stacks?source=first-run#runtime', undefined, '/stacks?source=first-run#runtime', 'Stacks page'],
    ['/setup', '/login', '/', 'Platforms page'],
    ['/setup', '/setup', '/', 'Platforms page'],
    ['/setup', '/login/mfa', '/', 'Platforms page'],
    ['/setup', '/login/mfa/setup', '/', 'Platforms page'],
  ] as const)(
    'opens the authenticated page after setup from %s with saved target %s',
    async (entry, savedTarget, expectedPath, page) => {
      if (savedTarget) sessionStorage.setItem('redirectTo', savedTarget);
      const token = `e30.${btoa(JSON.stringify({ exp: Math.floor(Date.now() / 1000) + 3600 }))}.signature`;
      const refresh = vi.fn(() => HttpResponse.json({ accessToken: token }));
      server.use(
        http.get('http://localhost/api/v1/setup/status', () => HttpResponse.json({ requiresSetup: true })),
        http.post('http://localhost/api/v1/setup/initialize', () =>
          HttpResponse.json({ accessToken: token, nextStep: LoginNextStep.Completed }),
        ),
        http.get('http://localhost/api/v1/authentication/refresh', refresh),
      );
      renderSetup(entry);
      await submitAdministrator();
      expect(await screen.findByText(page)).toBeVisible();
      await waitFor(() => expect(screen.getByTestId('path').textContent).toBe(expectedPath));
      expect(refresh).not.toHaveBeenCalled();
      expect(sessionStorage.getItem('redirectTo')).toBeNull();
    },
  );

  it.each([
    [LoginNextStep.EnrollMfa, 'MFA enrollment'],
    [LoginNextStep.VerifyMfa, 'MFA verification'],
  ] as const)('still requires %s when setup does not return an authenticated session', async (nextStep, page) => {
    server.use(
      http.get('http://localhost/api/v1/setup/status', () => HttpResponse.json({ requiresSetup: true })),
      http.post('http://localhost/api/v1/setup/initialize', () => HttpResponse.json({ nextStep })),
    );
    renderSetup('/login');
    await submitAdministrator();
    expect(await screen.findByText(page)).toBeVisible();
    expect(screen.queryByText('Platforms page')).not.toBeInTheDocument();
  });

  it('shows login when another request has already completed setup', async () => {
    server.use(
      http.get('http://localhost/api/v1/setup/status', () => HttpResponse.json({ requiresSetup: true })),
      http.post('http://localhost/api/v1/setup/initialize', () =>
        HttpResponse.json({ type: 'setup_already_complete', status: 409 }, { status: 409 }),
      ),
    );
    renderSetup('/login');
    await submitAdministrator();
    expect(await screen.findByText('Login page')).toBeVisible();
    expect(screen.queryByText('Platforms page')).not.toBeInTheDocument();
  });
});
