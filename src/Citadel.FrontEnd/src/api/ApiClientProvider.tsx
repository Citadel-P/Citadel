import { Api } from './generated/api.types';
import { ApiClientContext } from './ApiClientContext';

export const ApiClientProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const apiClient = new Api({
    baseUrl: import.meta.env.VITE_API_BASE_URL,
    baseApiParams: { secure: true, format: 'json', credentials: 'include' },
    securityWorker: (accessToken) => (accessToken ? { headers: { Authorization: `Bearer ${accessToken}` } } : {}),
  });

  return (
    <ApiClientContext.Provider
      value={{
        apiClient,
      }}>
      {children}
    </ApiClientContext.Provider>
  );
};
