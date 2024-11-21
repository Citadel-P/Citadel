import { createContext, useEffect, useState } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';

interface IContext {
  jwtToken: string;
  setJwtToken: (token: string) => void;
  logout: () => void;
  isAuthenticated: boolean;
}
interface IProps {
  children?: React.ReactNode;
}

const AuthContext = createContext<IContext | undefined>(undefined);

const jwtKey = 'jwt';
const storedJwt = localStorage.getItem(jwtKey);

const AuthProvider: React.FC<IProps> = ({ children }) => {
  const [jwtToken, setJwtToken] = useState(storedJwt ? JSON.parse(storedJwt) : '');

  useEffect(() => {
    window.localStorage.setItem(jwtKey, JSON.stringify(jwtToken));
  }, [jwtToken]);

  const logout = () => {
    window.localStorage.removeItem(jwtKey);
    window.location.href = '/';
  };

  const isTokenExpired = (token: string) => {
    if (!token) return true;
    const arrayToken = token.split('.');
    const tokenPayload = JSON.parse(atob(arrayToken[1]));
    return Math.floor(new Date().getTime() / 1000) >= tokenPayload?.sub;
  };

  return (
    <AuthContext.Provider
      value={{
        jwtToken,
        logout,
        setJwtToken,
        isAuthenticated: !isTokenExpired(jwtToken),
      }}>
      {children}
    </AuthContext.Provider>
  );
};

export default AuthProvider;

export const useAuthContext = () => useRequiredContext(AuthContext);
