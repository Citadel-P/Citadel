import { createContext } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { Api } from './_generated';
import { useAuthContext } from '@/features/login/AuthProvider';

interface IContext {
  apiClient: Api<unknown>;
}

interface IProps {
  children?: React.ReactNode;
}

const ApiClientContext = createContext<IContext | undefined>(undefined);

const ApiClientProvider: React.FC<IProps> = ({ children }) => {
  const { jwtToken } = useAuthContext();

  const apiClient = new Api({
    baseUrl: import.meta.env.VITE_API_BASE_URL,
    baseApiParams: { secure: true, format: 'json' },
    securityWorker: (accessToken) => (accessToken ? { headers: { Authorization: `Bearer ${accessToken}` } } : {}),
  });

  apiClient.setSecurityData(jwtToken);

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
