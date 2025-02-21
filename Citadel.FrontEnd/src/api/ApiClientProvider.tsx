import { createContext } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { Api } from './_generated';
import AuthProvider from '@/features/auth/AuthProvider';

interface IContext {
  apiClient: Api<unknown>;
}

interface IProps {
  children?: React.ReactNode;
}

const ApiClientContext = createContext<IContext | undefined>(undefined);

const ApiClientProvider: React.FC<IProps> = ({ children }) => {
  const apiClient = new Api({
    baseUrl: import.meta.env.VITE_API_BASE_URL,
    baseApiParams: { secure: true, format: 'json', credentials: 'include' },
    securityWorker: (accessToken) => (accessToken ? { headers: { Authorization: `Bearer ${accessToken}` } } : {}),
  });

  function onAccessTokenChange(accessToken: string | undefined) {
    apiClient.setSecurityData(accessToken);
  }

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
