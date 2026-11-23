import { Unplug, Wrench } from 'lucide-react';
import { ErrorPanel } from '@/components/custom/page-error';
import Loader from '@/components/ui/loader';
import { Button } from '@/components/ui/button';
import { Navigate, Outlet, useLocation } from 'react-router';
import { getPostLoginRedirect, REDIRECT_TO_KEY } from '@/features/auth/auth-route-guards';
import { useAuthContext } from '@/features/auth/auth-context';
import { hasMfaFlowStep } from '@/features/auth/mfa/mfa-flow';
import { SetupError, useSetupContext } from './setup-context';

function SetupUnavailable({ error, retry }: { error: SetupError; retry: () => void }) {
  const Icon = error.maintenance ? Wrench : Unplug;
  return (
    <ErrorPanel
      title={error.title}
      message={error.message}
      illustration={<Icon aria-hidden="true" className="my-6 size-24 text-muted-foreground" strokeWidth={1.25} />}>
      <Button type="button" onClick={retry}>
        Try again
      </Button>
    </ErrorPanel>
  );
}

export function RequireSetup() {
  const { isSetupReady, requiresSetup, error, retry } = useSetupContext();
  const { isAuthenticated, isAuthReady } = useAuthContext();

  if (!isSetupReady) return <Loader />;
  if (error) return <SetupUnavailable error={error} retry={retry} />;
  if (requiresSetup) return <Outlet />;
  if (!isAuthReady) return <Loader />;

  const redirectTo = getPostLoginRedirect();
  if (isAuthenticated) return <Navigate to={redirectTo} replace />;
  if (hasMfaFlowStep('setup')) return <Navigate to="/login/mfa/setup" replace />;
  if (hasMfaFlowStep('verify')) return <Navigate to="/login/mfa" replace />;
  return <Navigate to="/login" replace />;
}

export function RequireSetupComplete() {
  const { isSetupReady, requiresSetup, error, retry } = useSetupContext();
  const location = useLocation();

  if (!isSetupReady) return <Loader />;
  if (error) return <SetupUnavailable error={error} retry={retry} />;

  if (requiresSetup) {
    const currentUrl = `${location.pathname}${location.search}${location.hash}`;
    if (currentUrl !== '/setup') {
      sessionStorage.setItem(REDIRECT_TO_KEY, getPostLoginRedirect(currentUrl || '/'));
    }
    return <Navigate to="/setup" replace />;
  }

  return <Outlet />;
}
