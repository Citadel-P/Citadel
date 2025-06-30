import { createContext } from 'react';
import { Api } from './_generated';
import { useRequiredContext } from '@/hooks/useRequiredContext';

interface IContext {
  apiClient: Api<unknown>;
}

interface IProps {
  children?: React.ReactNode;
}

export const ApiClientContext = createContext<IContext | undefined>(undefined);

const ApiClientProvider: React.FC<IProps> = ({ children }) => {
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

export default ApiClientProvider;
export const useApiClientContext = () => useRequiredContext(ApiClientContext);
