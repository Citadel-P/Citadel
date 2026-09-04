import { ApiClientContext } from '@/api/api-client-context';
import { createApiClient } from '@/api/api-client-provider';
import { AuthContext, AuthContextValue } from '@/features/auth/auth-context';
import { createQueryClient } from '@/query-client-wrapper';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, RenderOptions } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { PropsWithChildren, ReactElement } from 'react';
import { MemoryRouter } from 'react-router';
import { RealtimeProvider } from '@/lib/context/realtime-provider';
import { ComponentProps } from 'react';
import { SetupContext, SetupContextValue } from '@/features/setup/setup-context';

const defaultAuth: AuthContextValue = {
  accessToken: 'test-access-token',
  isAuthenticated: true,
  isAuthReady: true,
  isPending: false,
  validationErrors: undefined,
  logout: () => {},
  login: async () => undefined,
  completeLogin: () => {},
};

const defaultSetup: SetupContextValue = {
  isSetupReady: true,
  requiresSetup: false,
  error: undefined,
  markSetupComplete: () => {},
  retry: () => {},
};

type RenderCitadelOptions = Omit<RenderOptions, 'wrapper'> & {
  route?: string;
  auth?: Partial<AuthContextValue>;
  setup?: Partial<SetupContextValue>;
  queryClient?: QueryClient;
  signalR?: Omit<ComponentProps<typeof RealtimeProvider>, 'children'>;
};

export function renderCitadel(ui: ReactElement, options: RenderCitadelOptions = {}) {
  const { route = '/', auth, setup, queryClient = createQueryClient(), signalR, ...renderOptions } = options;
  const apiClient = createApiClient('http://localhost');
  const authValue = { ...defaultAuth, ...auth };
  const setupValue = { ...defaultSetup, ...setup };

  const Wrapper = ({ children }: PropsWithChildren) => (
    <MemoryRouter initialEntries={[route]}>
      <QueryClientProvider client={queryClient}>
        <ApiClientContext.Provider value={{ apiClient }}>
          <SetupContext.Provider value={setupValue}>
            <AuthContext.Provider value={authValue}>
              {signalR ? (
                <RealtimeProvider realtimeTransport="SignalR" {...signalR}>
                  {children}
                </RealtimeProvider>
              ) : (
                children
              )}
            </AuthContext.Provider>
          </SetupContext.Provider>
        </ApiClientContext.Provider>
      </QueryClientProvider>
    </MemoryRouter>
  );

  return {
    user: userEvent.setup(),
    queryClient,
    apiClient,
    ...render(ui, { wrapper: Wrapper, ...renderOptions }),
  };
}
