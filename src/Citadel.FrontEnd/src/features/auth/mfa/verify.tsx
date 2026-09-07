import { AlertMessage } from '@/components/custom/alert-message';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { getPostLoginRedirect, REDIRECT_TO_KEY } from '@/features/auth/auth-route-guards';
import { useMutate } from '@/lib/hooks';
import { ArrowLeft, KeyRound, LoaderCircle, ShieldCheck, Smartphone } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import { useAuthContext } from '../auth-context';
import { MfaAuthShell } from './components';
import { clearMfaFlowStep, hasMfaFlowStep } from './mfa-flow';

type VerificationMode = 'totp' | 'recovery';

export default function MfaVerify() {
  const navigate = useNavigate();
  const { completeLogin } = useAuthContext();
  const verify = useMutate('verifyAuthenticationMfa');
  const hasPendingChallenge = hasMfaFlowStep('verify');
  const [mode, setMode] = useState<VerificationMode>('totp');
  const [code, setCode] = useState('');
  const [recoveryCode, setRecoveryCode] = useState('');
  const canVerifyTotp = /^\d{6}$/.test(code);
  const canVerifyRecoveryCode = recoveryCode.trim().length > 0;

  useEffect(() => {
    if (!hasPendingChallenge) {
      navigate('/login', { replace: true });
    }
  }, [hasPendingChallenge, navigate]);

  const enterApp = (accessToken: string) => {
    completeLogin(accessToken);
    clearMfaFlowStep();
    const redirectTo = getPostLoginRedirect();
    sessionStorage.removeItem(REDIRECT_TO_KEY);
    navigate(redirectTo, { replace: true });
  };

  const returnToLogin = () => {
    clearMfaFlowStep();
    navigate('/login', { replace: true });
  };

  if (!hasPendingChallenge) return null;

  return (
    <MfaAuthShell
      title="Verify sign-in"
      description="Use your authenticator app or a saved recovery code to finish signing in."
      panelClassName="sm:max-w-lg">
      <div className="space-y-5">
        <div className="flex gap-3 rounded-sm border bg-muted/20 p-4">
          <div className="flex size-10 shrink-0 items-center justify-center rounded-sm bg-primary/10 text-primary">
            <ShieldCheck className="size-5" />
          </div>
          <div className="space-y-1">
            <p className="text-sm font-medium">Second verification required</p>
            <p className="text-sm leading-6 text-muted-foreground">
              Enter the current six-digit TOTP code from your authenticator app. Recovery codes are available if you
              cannot access the app.
            </p>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-1 rounded-sm border bg-muted/30 p-1">
          <Button
            type="button"
            variant={mode === 'totp' ? 'secondary' : 'ghost'}
            className="justify-center rounded-sm"
            disabled={verify.isPending}
            onClick={() => setMode('totp')}>
            <Smartphone className="size-4" />
            Authenticator
          </Button>
          <Button
            type="button"
            variant={mode === 'recovery' ? 'secondary' : 'ghost'}
            className="justify-center rounded-sm"
            disabled={verify.isPending}
            onClick={() => setMode('recovery')}>
            <KeyRound className="size-4" />
            Recovery code
          </Button>
        </div>

        {verify.validationErrors && <AlertMessage type="warning">{verify.validationErrors}</AlertMessage>}

        {mode === 'recovery' ? (
          <form
            className="space-y-4"
            onSubmit={async (event) => {
              event.preventDefault();
              if (!canVerifyRecoveryCode) return;
              const response = await verify.mutateAsync({ data: { recoveryCode: recoveryCode.trim() } });
              enterApp(response.data.accessToken);
            }}>
            <div className="space-y-2">
              <label className="text-sm font-medium leading-none" htmlFor="mfa-recovery-code">
                Recovery code
              </label>
              <Input
                id="mfa-recovery-code"
                value={recoveryCode}
                onChange={(event) => setRecoveryCode(event.target.value)}
                placeholder="ABCD-EFGH-IJKL"
                autoComplete="one-time-code"
                disabled={verify.isPending}
              />
              <p className="text-xs leading-5 text-muted-foreground">
                Use one unused recovery code from your two-factor setup.
              </p>
            </div>
            <Button type="submit" className="w-full" disabled={!canVerifyRecoveryCode || verify.isPending}>
              Verify recovery code
              {verify.isPending && <LoaderCircle className="ml-1 size-4 animate-spin" />}
            </Button>
          </form>
        ) : (
          <form
            className="space-y-4"
            onSubmit={async (event) => {
              event.preventDefault();
              if (!canVerifyTotp) return;
              const response = await verify.mutateAsync({ data: { code } });
              enterApp(response.data.accessToken);
            }}>
            <div className="space-y-2">
              <label className="text-sm font-medium leading-none" htmlFor="mfa-code">
                Authenticator code
              </label>
              <Input
                id="mfa-code"
                inputMode="numeric"
                autoComplete="one-time-code"
                maxLength={6}
                value={code}
                onChange={(event) => setCode(event.target.value.replace(/\D/g, '').slice(0, 6))}
                placeholder="000000"
                className="h-12 text-center font-mono text-lg"
                disabled={verify.isPending}
              />
              <p className="text-xs leading-5 text-muted-foreground">
                Open your authenticator app and enter the current TOTP code.
              </p>
            </div>
            <Button type="submit" className="w-full" disabled={!canVerifyTotp || verify.isPending}>
              Verify code
              {verify.isPending && <LoaderCircle className="ml-1 size-4 animate-spin" />}
            </Button>
          </form>
        )}

        <Button type="button" variant="outline" className="w-full" disabled={verify.isPending} onClick={returnToLogin}>
          <ArrowLeft className="size-4" />
          Back to login
        </Button>
      </div>
    </MfaAuthShell>
  );
}
