import { createContext, useEffect, useState } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { Api, ProblemDetails } from './_generated';
import { useMutation } from '@tanstack/react-query';
import { useQueryClientContext } from '@/QueryClientWrapper';
import { toast } from 'sonner';

interface IContext {
  apiClient: Api<unknown>;
  accessToken: string | undefined;
  isAuthenticated: boolean;
}

interface IProps {
  children?: React.ReactNode;
}

const ApiClientContext = createContext<IContext | undefined>(undefined);

const ApiClientProvider: React.FC<IProps> = ({ children }) => {
  const { error } = useQueryClientContext();
  const [accessToken, setAccessToken] = useState<string | undefined>(undefined);

  const apiClient = new Api({
    baseUrl: import.meta.env.VITE_API_BASE_URL,
    baseApiParams: { secure: true, format: 'json', credentials: 'include' },
    securityWorker: (accessToken) => (accessToken ? { headers: { Authorization: `Bearer ${accessToken}` } } : {}),
  });

  const { mutate: requestRefreshToken, isSuccess, data } = useMutation({ mutationFn: apiClient.api.authenticationRefreshToken });
  const { mutate: logout } = useMutation({ mutationFn: apiClient.api.authenticationLogout });
  if (isSuccess && data?.data.accessToken) {
    apiClient.setSecurityData(data?.data.accessToken);
  }

  useEffect(() => {
    // Request a new access token when the component mounts
    requestRefreshToken(undefined);
  }, []);

  useEffect(() => {
    if (isSuccess && data?.data.accessToken) {
      setAccessToken(data?.data.accessToken);
    }
  }, [data]);

  useEffect(() => {
    if (error?.status === 401) {
      logout(undefined);
      window.location.href = '/login';
    } else if (error?.status != null && error?.status >= 500) {
      const problem = error.error as ProblemDetails;
      toast.error(problem.status + ' ' + problem.title, {
        description: problem.detail,
      });
    }
  }, [error]);

  return (
    <ApiClientContext.Provider
      value={{
        apiClient,
        accessToken,
        isAuthenticated: accessToken !== undefined,
      }}>
      {children}
    </ApiClientContext.Provider>
  );
};

export default ApiClientProvider;

export const useApiClientContext = () => useRequiredContext(ApiClientContext);
