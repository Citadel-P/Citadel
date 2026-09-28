import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Separator } from '@/components/ui/separator';
import { useMutate, useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { CheckCircle2, FingerprintPattern, KeyRound, RefreshCw, ShieldAlert, ShieldCheck, ShieldOff, XCircle } from 'lucide-react';
import { FormEvent, ReactNode, useState } from 'react';
import { toast } from 'sonner';
import { RecoveryCodesPanel, TotpSetupPanel } from '@/features/auth/mfa/components';

type SetupState =
  | { step: 'password'; password: string }
  | { step: 'code'; secret: string; otpAuthUri: string }
  | { step: 'recovery'; codes: string[] };

export function ProfileMfaCommand({ canUseLocalPassword }: { canUseLocalPassword: boolean }) {
  const queryClient = useQueryClient();
  const statusQuery = useRead('getProfileMfaStatus');
  const startSetup = useMutate('startProfileMfaSetup');
  const confirmSetup = useMutate('confirmProfileMfaSetup');
  const disableMfa = useMutate('disableProfileMfa');
  const regenerateCodes = useMutate('regenerateProfileMfaRecoveryCodes');

  const status = statusQuery.data?.data;
  const [setup, setSetup] = useState<SetupState | null>(null);
  const [disableOpen, setDisableOpen] = useState(false);
  const [regenerateOpen, setRegenerateOpen] = useState(false);
  const [recoveryCodes, setRecoveryCodes] = useState<string[] | null>(null);

  const refreshStatus = async () => {
    await queryClient.invalidateQueries({ queryKey: ['getProfileMfaStatus'] });
    await queryClient.invalidateQueries({ queryKey: ['listProfileSessions'] });
  };

  if (statusQuery.isLoading) return <MfaLoadingPanel />;
  if (!canUseLocalPassword) return <MfaUnavailablePanel />;

  const enabled = status?.enabled === true;
  const remainingRecoveryCodes = Number(status?.remainingRecoveryCodes ?? 0);
  const recoveryState = getRecoveryState(enabled, remainingRecoveryCodes);

  return (
    <div className="max-w-full overflow-hidden rounded-sm border bg-background">
      <div className="flex flex-col gap-4 p-4 lg:flex-row lg:items-start lg:justify-between">
        <div className="flex min-w-0 gap-3">
          <div
            className={`flex size-10 shrink-0 items-center justify-center rounded-sm border ${
              enabled
                ? 'border-green-500/30 bg-green-500/10 text-green-700 dark:text-green-300'
                : 'border-border bg-muted/40 text-muted-foreground'
            }`}>
            {enabled ? <ShieldCheck className="size-5" /> : <ShieldOff className="size-5" />}
          </div>
          <div className="min-w-0">
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <p className="text-sm font-semibold">Authenticator app</p>
              <Badge
                variant={enabled ? 'secondary' : 'outline'}
                className={`rounded-sm ${enabled ? 'bg-green-500/15 text-green-700 dark:text-green-300' : 'bg-muted/40 text-muted-foreground'}`}>
                {enabled ? 'Enabled' : 'Disabled'}
              </Badge>
            </div>
            <p className="mt-1 max-w-xl text-sm text-muted-foreground">
              {enabled
                ? 'Local-password sign-in requires a six-digit authenticator code.'
                : 'Add a second verification step for local-password sign-in.'}
            </p>
          </div>
        </div>

        <div className="flex shrink-0 flex-wrap gap-2 lg:justify-end">
          {!enabled && (
            <Button type="button" size="sm" onClick={() => setSetup({ step: 'password', password: '' })}>
              <FingerprintPattern className="size-4" />
              Enable
            </Button>
          )}
          {enabled && (
            <>
              <Button type="button" size="sm" variant="outline" onClick={() => setRegenerateOpen(true)}>
                <RefreshCw className="size-4" />
                Recovery Codes
              </Button>
              <Button type="button" size="sm" variant="outline" disabled={!status?.canDisable} onClick={() => setDisableOpen(true)}>
                <ShieldOff className="size-4" />
                Disable
              </Button>
            </>
          )}
        </div>
      </div>

      <Separator />

      <div className="grid gap-px bg-border sm:grid-cols-3">
        <MfaSummaryCell
          icon={enabled ? <CheckCircle2 className="size-4" /> : <XCircle className="size-4" />}
          label="Status"
          value={enabled ? 'Protected' : 'Not configured'}
          tone={enabled ? 'success' : 'muted'}
        />
        <MfaSummaryCell
          icon={<ShieldAlert className="size-4" />}
          label="Policy"
          value={formatPolicy(status?.policy)}
          tone={status?.canDisable === false ? 'warning' : 'muted'}
        />
        <MfaSummaryCell
          icon={<KeyRound className="size-4" />}
          label="Recovery codes"
          value={recoveryState.label}
          tone={recoveryState.tone}
        />
      </div>

      {enabled && status?.canDisable === false && (
        <>
          <Separator />
          <div className="bg-amber-500/10 px-4 py-3 text-sm text-amber-800 dark:text-amber-200">
            MFA is required by the active policy and cannot be disabled for this account.
          </div>
        </>
      )}

      <SetupDialog
        setup={setup}
        pending={startSetup.isPending || confirmSetup.isPending}
        error={startSetup.validationErrors || confirmSetup.validationErrors}
        onClose={() => setSetup(null)}
        onSubmitPassword={async (password) => {
          const response = await startSetup.mutateAsync({ data: { password } });
          setSetup({ step: 'code', secret: response.data.secret, otpAuthUri: response.data.otpAuthUri });
        }}
        onSubmitCode={async (code) => {
          const response = await confirmSetup.mutateAsync({ data: { code } });
          setSetup({ step: 'recovery', codes: [...response.data.recoveryCodes] });
          await refreshStatus();
          toast.success('Two-factor authentication enabled.');
        }}
        onFinish={() => setSetup(null)}
      />

      <DisableDialog
        open={disableOpen}
        pending={disableMfa.isPending}
        error={disableMfa.validationErrors}
        onOpenChange={setDisableOpen}
        onConfirm={async (values) => {
          await disableMfa.mutateAsync({ data: values });
          setDisableOpen(false);
          await refreshStatus();
          toast.success('Two-factor authentication disabled.');
        }}
      />

      <RegenerateDialog
        open={regenerateOpen}
        pending={regenerateCodes.isPending}
        error={regenerateCodes.validationErrors}
        onOpenChange={setRegenerateOpen}
        onConfirm={async (values) => {
          const response = await regenerateCodes.mutateAsync({ data: values });
          setRegenerateOpen(false);
          setRecoveryCodes([...response.data.recoveryCodes]);
          await refreshStatus();
          toast.success('Recovery codes regenerated.');
        }}
      />

      <Dialog open={Boolean(recoveryCodes)} onOpenChange={(open) => !open && setRecoveryCodes(null)}>
        <DialogContent className="sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>Save recovery codes</DialogTitle>
            <DialogDescription>These codes replace the previous set and are shown only once.</DialogDescription>
          </DialogHeader>
          {recoveryCodes && <RecoveryCodesPanel codes={recoveryCodes} actionLabel="Done" onContinue={() => setRecoveryCodes(null)} />}
        </DialogContent>
      </Dialog>
    </div>
  );
}

function MfaLoadingPanel() {
  return (
    <div className="max-w-3xl rounded-sm border bg-background p-4">
      <div className="flex items-center gap-3">
        <div className="size-10 animate-pulse rounded-sm bg-muted" />
        <div className="min-w-0 flex-1 space-y-2">
          <div className="h-4 w-40 animate-pulse rounded-sm bg-muted" />
          <div className="h-3 w-72 max-w-full animate-pulse rounded-sm bg-muted" />
        </div>
      </div>
    </div>
  );
}

function MfaUnavailablePanel() {
  return (
    <div className="max-w-3xl rounded-sm border bg-muted/20 p-4">
      <div className="flex gap-3">
        <div className="flex size-10 shrink-0 items-center justify-center rounded-sm border bg-background text-muted-foreground">
          <ShieldOff className="size-5" />
        </div>
        <div className="min-w-0">
          <p className="text-sm font-semibold">Authenticator app unavailable</p>
          <p className="mt-1 text-sm text-muted-foreground">Citadel MFA applies to accounts that can sign in with a local password.</p>
        </div>
      </div>
    </div>
  );
}

function MfaSummaryCell({
  icon,
  label,
  value,
  tone,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  tone: 'success' | 'warning' | 'muted';
}) {
  const toneClass =
    tone === 'success'
      ? 'text-green-700 dark:text-green-300'
      : tone === 'warning'
        ? 'text-amber-800 dark:text-amber-200'
        : 'text-muted-foreground';

  return (
    <div className="bg-background px-4 py-3">
      <div className={`flex items-center gap-2 ${toneClass}`}>
        {icon}
        <span className="text-xs font-medium uppercase">{label}</span>
      </div>
      <p className="mt-1 truncate text-sm font-medium" title={value}>
        {value}
      </p>
    </div>
  );
}

function getRecoveryState(enabled: boolean, remainingCodes: number): { label: string; tone: 'success' | 'warning' | 'muted' } {
  if (!enabled) return { label: '-', tone: 'muted' };
  if (remainingCodes <= 0) return { label: 'No codes remaining', tone: 'warning' };
  if (remainingCodes <= 2) return { label: `${remainingCodes} remaining`, tone: 'warning' };
  return { label: `${remainingCodes} remaining`, tone: 'success' };
}

function formatPolicy(policy?: string) {
  if (!policy) return 'Optional';
  return policy.replace(/([a-z])([A-Z])/g, '$1 $2');
}

function SetupDialog({
  setup,
  pending,
  error,
  onClose,
  onSubmitPassword,
  onSubmitCode,
  onFinish,
}: {
  setup: SetupState | null;
  pending: boolean;
  error?: string;
  onClose: () => void;
  onSubmitPassword: (password: string) => Promise<void>;
  onSubmitCode: (code: string) => Promise<void>;
  onFinish: () => void;
}) {
  return (
    <Dialog open={Boolean(setup)} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className={setup?.step === 'code' || setup?.step === 'recovery' ? 'sm:max-w-xl' : undefined}>
        <DialogHeader>
          <DialogTitle>Enable two-factor authentication</DialogTitle>
          <DialogDescription>
            {setup?.step === 'password' && 'Confirm your password before creating a new authenticator setup.'}
            {setup?.step === 'code' && 'Use your authenticator app to scan the QR code, then enter the six-digit TOTP code.'}
            {setup?.step === 'recovery' && 'Save these recovery codes before closing this dialog.'}
          </DialogDescription>
        </DialogHeader>
        {setup?.step === 'password' && (
          <PasswordStep
            pending={pending}
            error={error}
            submitLabel="Continue"
            onSubmit={async (password) => onSubmitPassword(password)}
          />
        )}
        {setup?.step === 'code' && (
          <SetupCodeStep
            secret={setup.secret}
            otpAuthUri={setup.otpAuthUri}
            pending={pending}
            error={error}
            onClose={onClose}
            onSubmit={onSubmitCode}
          />
        )}
        {setup?.step === 'recovery' && <RecoveryCodesPanel codes={setup.codes} actionLabel="Done" onContinue={onFinish} />}
      </DialogContent>
    </Dialog>
  );
}

function SetupCodeStep({
  secret,
  otpAuthUri,
  pending,
  error,
  onClose,
  onSubmit,
}: {
  secret: string;
  otpAuthUri: string;
  pending: boolean;
  error?: string;
  onClose: () => void;
  onSubmit: (code: string) => Promise<void>;
}) {
  const [code, setCode] = useState('');
  return (
    <form
      className="space-y-4"
      onSubmit={async (event) => {
        event.preventDefault();
        if (/^\d{6}$/.test(code)) await onSubmit(code);
      }}>
      <div className="space-y-3 rounded-sm border bg-muted/20 p-3">
        <div className="flex gap-3">
          <span className="flex size-6 shrink-0 items-center justify-center rounded-sm bg-primary text-xs font-semibold text-primary-foreground">
            1
          </span>
          <div className="min-w-0">
            <p className="text-sm font-medium">Scan the QR code</p>
            <p className="text-sm text-muted-foreground">
              Add this Citadel account in your authenticator app. Use the manual secret if the QR code cannot be scanned.
            </p>
          </div>
        </div>
        <TotpSetupPanel secret={secret} otpAuthUri={otpAuthUri} />
      </div>

      <div className="space-y-3 rounded-sm border bg-background p-3">
        <div className="flex gap-3">
          <span className="flex size-6 shrink-0 items-center justify-center rounded-sm bg-primary text-xs font-semibold text-primary-foreground">
            2
          </span>
          <div className="min-w-0">
            <p className="text-sm font-medium">Enter the TOTP code</p>
            <p className="text-sm text-muted-foreground">Type the current six-digit code shown by your authenticator app.</p>
          </div>
        </div>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <div className="space-y-2">
          <label className="text-sm font-medium leading-none" htmlFor="profile-mfa-code">
            Authenticator code
          </label>
          <Input
            id="profile-mfa-code"
            value={code}
            onChange={(event) => setCode(event.target.value.replace(/\D/g, '').slice(0, 6))}
            inputMode="numeric"
            autoComplete="one-time-code"
            placeholder="000000"
            maxLength={6}
            disabled={pending}
          />
        </div>
      </div>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onClose} disabled={pending}>
          Cancel
        </Button>
        <Button type="submit" disabled={!/^\d{6}$/.test(code) || pending}>
          Confirm
        </Button>
      </DialogFooter>
    </form>
  );
}

function PasswordStep({
  pending,
  error,
  submitLabel,
  onSubmit,
}: {
  pending: boolean;
  error?: string;
  submitLabel: string;
  onSubmit: (password: string) => Promise<void>;
}) {
  const [password, setPassword] = useState('');
  return (
    <form
      className="space-y-4"
      onSubmit={async (event: FormEvent) => {
        event.preventDefault();
        if (password) await onSubmit(password);
      }}>
      {error && <p className="text-sm text-destructive">{error}</p>}
      <Input
        type="password"
        value={password}
        autoComplete="current-password"
        placeholder="Current password"
        onChange={(event) => setPassword(event.target.value)}
        disabled={pending}
      />
      <DialogFooter>
        <Button type="submit" disabled={!password || pending}>
          {submitLabel}
        </Button>
      </DialogFooter>
    </form>
  );
}

function DisableDialog({
  open,
  pending,
  error,
  onOpenChange,
  onConfirm,
}: {
  open: boolean;
  pending: boolean;
  error?: string;
  onOpenChange: (open: boolean) => void;
  onConfirm: (values: { password: string; code?: string; recoveryCode?: string }) => Promise<void>;
}) {
  const [password, setPassword] = useState('');
  const [code, setCode] = useState('');
  const [recoveryCode, setRecoveryCode] = useState('');
  const hasCode = Boolean(code);
  const hasRecoveryCode = Boolean(recoveryCode);
  const canSubmit = Boolean(password && hasCode !== hasRecoveryCode);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Disable two-factor authentication</DialogTitle>
          <DialogDescription>Confirm with your password and either an authenticator code or a recovery code.</DialogDescription>
        </DialogHeader>
        <form
          className="space-y-3"
          onSubmit={async (event) => {
            event.preventDefault();
            await onConfirm({ password, code: code || undefined, recoveryCode: recoveryCode || undefined });
            setPassword('');
            setCode('');
            setRecoveryCode('');
          }}>
          {error && <p className="text-sm text-destructive">{error}</p>}
          <Input
            type="password"
            value={password}
            autoComplete="current-password"
            placeholder="Current password"
            onChange={(event) => setPassword(event.target.value)}
            disabled={pending}
          />
          <Input
            value={code}
            inputMode="numeric"
            placeholder="Authenticator code"
            maxLength={6}
            onChange={(event) => setCode(event.target.value.replace(/\D/g, '').slice(0, 6))}
            disabled={pending || hasRecoveryCode}
          />
          <Input
            value={recoveryCode}
            placeholder="Recovery code"
            onChange={(event) => setRecoveryCode(event.target.value)}
            disabled={pending || hasCode}
          />
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={pending}>
              Cancel
            </Button>
            <Button type="submit" disabled={!canSubmit || pending}>
              Disable
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function RegenerateDialog({
  open,
  pending,
  error,
  onOpenChange,
  onConfirm,
}: {
  open: boolean;
  pending: boolean;
  error?: string;
  onOpenChange: (open: boolean) => void;
  onConfirm: (values: { password: string; code: string }) => Promise<void>;
}) {
  const [password, setPassword] = useState('');
  const [code, setCode] = useState('');
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Regenerate recovery codes</DialogTitle>
          <DialogDescription>Creating a new set immediately invalidates every existing recovery code.</DialogDescription>
        </DialogHeader>
        <form
          className="space-y-3"
          onSubmit={async (event) => {
            event.preventDefault();
            if (/^\d{6}$/.test(code)) await onConfirm({ password, code });
            setPassword('');
            setCode('');
          }}>
          {error && <p className="text-sm text-destructive">{error}</p>}
          <Input type="password" value={password} placeholder="Current password" onChange={(event) => setPassword(event.target.value)} />
          <Input value={code} inputMode="numeric" placeholder="Authenticator code" maxLength={6} onChange={(event) => setCode(event.target.value.replace(/\D/g, '').slice(0, 6))} />
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={pending}>
              Cancel
            </Button>
            <Button type="submit" disabled={!password || !/^\d{6}$/.test(code) || pending}>
              Regenerate
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
