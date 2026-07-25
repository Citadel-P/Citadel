import { ApiClientContext } from '@/api/api-client-context';
import { createApiClient } from '@/api/api-client-provider';
import { AuthContext, AuthContextValue } from '@/features/auth/auth-context';
import { createQueryClient } from '@/query-client-wrapper';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, RenderOptions } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { PropsWithChildren, ReactElement } from 'react';
import { MemoryRouter } from 'react-router';
import { SignalRProvider } from '@/lib/context/signalr-provider';
import { ComponentProps } from 'react';

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

type RenderCitadelOptions = Omit<RenderOptions, 'wrapper'> & {
  route?: string;
  auth?: Partial<AuthContextValue>;
  queryClient?: QueryClient;
  signalR?: Omit<ComponentProps<typeof SignalRProvider>, 'children'>;
};

export function renderCitadel(ui: ReactElement, options: RenderCitadelOptions = {}) {
  const { route = '/', auth, queryClient = createQueryClient(), signalR, ...renderOptions } = options;
  const apiClient = createApiClient('http://localhost');
  const authValue = { ...defaultAuth, ...auth };

  const Wrapper = ({ children }: PropsWithChildren) => (
    <MemoryRouter initialEntries={[route]}>
      <QueryClientProvider client={queryClient}>
        <ApiClientContext.Provider value={{ apiClient }}>
          <AuthContext.Provider value={authValue}>
            {signalR ? <SignalRProvider {...signalR}>{children}</SignalRProvider> : children}
          </AuthContext.Provider>
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
