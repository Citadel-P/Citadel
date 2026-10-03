import Loader from '@/components/ui/loader';
import { useEffect } from 'react';
import { Navigate, Outlet, useLocation } from 'react-router';
import { useAuthContext } from './auth-context';

export const REDIRECT_TO_KEY = 'redirectTo';

export function getPostLoginRedirect(target = sessionStorage.getItem(REDIRECT_TO_KEY) ?? '/') {
  // Authentication/setup pages cannot be destinations for a signed-in user.
  return /^\/(?:login|setup)(?:[/?#]|$)/i.test(target) ? '/' : target;
}

export const RequireAuth = () => {
  const { isAuthenticated, isAuthReady, accessToken } = useAuthContext();
  const location = useLocation();

  useEffect(() => {
    if (isAuthenticated && accessToken) {
      sessionStorage.removeItem(REDIRECT_TO_KEY);
    }
  }, [accessToken, isAuthenticated]);

  if (!isAuthReady) {
    return <Loader />;
  }

  if (!isAuthenticated || !accessToken) {
    const currentUrl = `${location.pathname}${location.search}${location.hash}`;
    sessionStorage.setItem(REDIRECT_TO_KEY, currentUrl || '/');
    return <Navigate to="/login" replace />;
  }

  return <Outlet />;
};

export const RequireNoAuth = () => {
  const { isAuthenticated, isAuthReady } = useAuthContext();

  if (!isAuthReady) {
    return <Loader />;
  }

  if (isAuthenticated) {
    const redirectTo = getPostLoginRedirect();
    return <Navigate to={redirectTo} replace />;
  }

  return <Outlet />;
};
