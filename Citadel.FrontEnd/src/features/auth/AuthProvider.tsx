import { useEffect, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useGETAccessToken } from './useGETAccessToken';
import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { createContext } from 'use-context-selector';
import { useHTTPErrorHandler } from './useHTTPErrorHandler';

interface IContext {
  accessToken: string | undefined;
  isAuthenticated: boolean;
}

interface IProps {
  children?: React.ReactNode;
}

const accessTokenKey = 'access_token';
const storedJwt = sessionStorage.getItem(accessTokenKey);

export const AuthContext = createContext<IContext | undefined>(undefined);

const AuthProvider: React.FC<IProps> = ({ children }) => {
  useHTTPErrorHandler();
  const client = useQueryClient();
  const { data: accessTokenData, isSuccess } = useGETAccessToken();
  const apiClient = useContextSelector(ApiClientContext, (s) => s?.apiClient!);
  const [accessToken, setAccessToken] = useState<string | undefined>(storedJwt ?? undefined);
  let isAuthenticated = accessToken != null;

  useEffect(() => {
    if (isSuccess && accessTokenData?.data.accessToken) {
      setAccessToken(accessTokenData?.data.accessToken);
      apiClient.setSecurityData(accessTokenData?.data.accessToken);
      sessionStorage.setItem(accessTokenKey, accessTokenData?.data.accessToken!);
    }
  }, [accessTokenData]);

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout>;

    if (isAuthenticated && accessToken) {
      const result = parseJwt(accessToken);

      if (result?.exp) {
        const currentTime = Math.floor(Date.now() / 1000);
        const timeToExpire = (result.exp - currentTime) * 1000 - 10 * 1000;
        timer = setTimeout(
          () => {
            client.invalidateQueries({ queryKey: ['getAccessToken'] });
          },
          Math.max(0, timeToExpire),
        );
      }
    }

    return () => {
      clearTimeout(timer);
    };
  }, [accessToken, isAuthenticated]);

  const parseJwt = (token: string) => {
    if (!token) return true;
    const arrayToken = token.split('.');
    return JSON.parse(atob(arrayToken[1]));
  };

  return (
    <AuthContext.Provider
      value={{
        accessToken,
        isAuthenticated,
      }}>
      {children}
    </AuthContext.Provider>
  );
};

export default AuthProvider;
