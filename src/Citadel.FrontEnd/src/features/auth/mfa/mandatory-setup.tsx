import Loader from '@/components/ui/loader';
import { REDIRECT_TO_KEY } from '@/router';
import { useMutate, useRead } from '@/lib/hooks';
import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import { useAuthContext } from '../auth-context';
import { MfaAuthShell, RecoveryCodesPanel, TotpCodeForm, TotpSetupPanel } from './components';
import { clearMfaFlowStep, hasMfaFlowStep } from './mfa-flow';

export default function MandatoryMfaSetup() {
  const navigate = useNavigate();
  const { completeLogin } = useAuthContext();
  const hasPendingSetup = hasMfaFlowStep('setup');
  const setup = useRead('getAuthenticationMfaSetup', undefined, { enabled: hasPendingSetup });
  const confirm = useMutate('confirmAuthenticationMfaSetup');
  const [result, setResult] = useState<{ accessToken: string; recoveryCodes: string[] }>();

  useEffect(() => {
    if (!hasPendingSetup) {
      navigate('/login', { replace: true });
    }
  }, [hasPendingSetup, navigate]);

  useEffect(() => {
    if (hasPendingSetup && !setup.isLoading && !setup.data?.data) {
      clearMfaFlowStep();
    }
  }, [hasPendingSetup, setup.data?.data, setup.isLoading]);

  if (!hasPendingSetup) return null;

  if (setup.isLoading) return <Loader />;

  if (!setup.data?.data) {
    return (
      <MfaAuthShell title="Two-factor setup">
        <p className="text-sm text-muted-foreground">Your setup session expired.</p>
      </MfaAuthShell>
    );
  }

  if (result) {
    return (
      <MfaAuthShell title="Save recovery codes">
        <RecoveryCodesPanel
          codes={result.recoveryCodes}
          actionLabel="Continue"
          onContinue={() => {
            completeLogin(result.accessToken);
            clearMfaFlowStep();
            const redirectTo = sessionStorage.getItem(REDIRECT_TO_KEY) ?? '/';
            sessionStorage.removeItem(REDIRECT_TO_KEY);
            navigate(redirectTo, { replace: true });
          }}
        />
      </MfaAuthShell>
    );
  }

  return (
    <MfaAuthShell title="Set up two-factor authentication">
      <TotpSetupPanel secret={setup.data.data.secret} otpAuthUri={setup.data.data.otpAuthUri} />
      <TotpCodeForm
        pending={confirm.isPending}
        error={confirm.validationErrors}
        submitLabel="Confirm setup"
        onSubmit={async (code) => {
          const response = await confirm.mutateAsync({ data: { code } });
          setResult({ accessToken: response.data.accessToken, recoveryCodes: [...response.data.recoveryCodes] });
        }}
      />
    </MfaAuthShell>
  );
}
