import Loader from '@/components/ui/loader';
import { Button } from '@/components/ui/button';
import { Navigate, Outlet, useLocation } from 'react-router';
import { REDIRECT_TO_KEY } from '@/features/auth/auth-route-guards';
import { useAuthContext } from '@/features/auth/auth-context';
import { SetupError, useSetupContext } from './setup-context';

function SetupUnavailable({ error, retry }: { error: SetupError; retry: () => void }) {
  return (
    <div className="flex min-h-screen items-center justify-center bg-card px-6">
      <div className="w-full max-w-sm space-y-4 text-center">
        <h1 className="text-xl font-semibold">{error.title}</h1>
        <p className="text-sm text-muted-foreground">{error.message}</p>
        <Button type="button" variant="outline" onClick={retry}>
          Try again
        </Button>
      </div>
    </div>
  );
}

export function RequireSetup() {
  const { isSetupReady, requiresSetup, error, retry } = useSetupContext();
  const { isAuthenticated, isAuthReady } = useAuthContext();

  if (!isSetupReady) return <Loader />;
  if (error) return <SetupUnavailable error={error} retry={retry} />;
  if (requiresSetup) return <Outlet />;
  if (!isAuthReady) return <Loader />;

  const redirectTo = sessionStorage.getItem(REDIRECT_TO_KEY) ?? '/';
  return <Navigate to={isAuthenticated ? redirectTo : '/login'} replace />;
}

export function RequireSetupComplete() {
  const { isSetupReady, requiresSetup, error, retry } = useSetupContext();
  const location = useLocation();

  if (!isSetupReady) return <Loader />;
  if (error) return <SetupUnavailable error={error} retry={retry} />;

  if (requiresSetup) {
    const currentUrl = `${location.pathname}${location.search}${location.hash}`;
    if (currentUrl !== '/setup') {
      sessionStorage.setItem(REDIRECT_TO_KEY, currentUrl || '/');
    }
    return <Navigate to="/setup" replace />;
  }

  return <Outlet />;
}
