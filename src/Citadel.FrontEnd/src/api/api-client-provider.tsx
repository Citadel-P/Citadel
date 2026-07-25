import { Api } from './generated/api.types';
import { ApiClientContext } from './api-client-context';
import { useMemo } from 'react';

export const createApiClient = (baseUrl = import.meta.env.VITE_API_BASE_URL) =>
  new Api({
    baseUrl,
    baseApiParams: { secure: true, format: 'json', credentials: 'include' },
    securityWorker: (accessToken) => {
      return accessToken ? { headers: { Authorization: `Bearer ${accessToken}` } } : {};
    },
  });

export const ApiClientProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const apiClient = useMemo(() => createApiClient(), []);

  return <ApiClientContext.Provider value={{ apiClient }}>{children}</ApiClientContext.Provider>;
};
