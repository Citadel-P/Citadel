import { FieldSwitch } from '@/components/custom/form-builder';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { mfaPolicyOptions, mfaPolicyDescription } from '@/lib/mfa-policy';
import { FormEvent, useState } from 'react';
import { useNavigate } from 'react-router';
import { ChevronDown, Eye, EyeOff, LoaderCircle } from 'lucide-react';
import { LoginNextStep, ProblemDetails, MfaPolicy } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { getPostLoginRedirect } from '@/features/auth/auth-route-guards';
import { useAuthContext } from '@/features/auth/auth-context';
import { AuthShell } from '@/features/auth/auth-shell';
import { clearMfaFlowStep, markMfaFlowStep } from '@/features/auth/mfa/mfa-flow';
import { Constants } from '@/lib/constants';
import { useMutate } from '@/lib/hooks';
import { useSetupContext } from './setup-context';

type SetupForm = {
  name: string;
  email: string;
  password: string;
  confirmPassword: string;
};

type SetupErrors = Partial<Record<keyof SetupForm, string>>;

const initialForm: SetupForm = {
  name: '',
  email: '',
  password: '',
  confirmPassword: '',
};

export default function InitialSetup() {
  const navigate = useNavigate();
  const { completeLogin } = useAuthContext();
  const {
    markSetupComplete,
    passwordMinimumLength,
    passwordMaximumLength,
    updateChecksEnabled,
    updateChecksAllowed,
    mfaPolicy: initialMfaPolicy,
    minimumMfaPolicy,
  } = useSetupContext();
  const [checkForUpdates, setCheckForUpdates] = useState(updateChecksEnabled && updateChecksAllowed);
  const [mfaPolicy, setMfaPolicy] = useState(initialMfaPolicy);
  const initialize = useMutate('initializeCitadel');
  const [form, setForm] = useState(initialForm);
  const [errors, setErrors] = useState<SetupErrors>({});
  const [showPassword, setShowPassword] = useState(false);

  const updateField = (field: keyof SetupForm, value: string) => {
    setForm((current) => ({ ...current, [field]: value }));
    if (errors[field]) {
      setErrors((current) => ({ ...current, [field]: undefined }));
    }
  };

  const validate = () => {
    const next: SetupErrors = {};
    if (!form.name) {
      next.name = 'Username is required.';
    } else if (!new RegExp(Constants.validNameIdentifier).test(form.name)) {
      next.name = 'Use letters, numbers, hyphens, or underscores.';
    }

    if (!form.email) {
      next.email = 'Email address is required.';
    } else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(form.email)) {
      next.email = 'Enter a valid email address.';
    }

    if (!form.password) {
      next.password = 'Password is required.';
    } else if ([...form.password].length < passwordMinimumLength) {
      next.password = `Password must be at least ${passwordMinimumLength} characters.`;
    } else if ([...form.password].length > passwordMaximumLength) {
      next.password = `Password must be no more than ${passwordMaximumLength} characters.`;
    }

    if (!form.confirmPassword) {
      next.confirmPassword = 'Confirm your password.';
    } else if (form.password !== form.confirmPassword) {
      next.confirmPassword = 'Passwords do not match.';
    }

    setErrors(next);
    return Object.keys(next).length === 0;
  };

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!validate()) return;

    clearMfaFlowStep();
    let response;
    try {
      response = await initialize.mutateAsync({
        data: {
          name: form.name,
          email: form.email,
          password: form.password,
          updateChecksEnabled: checkForUpdates && updateChecksAllowed,
          mfaPolicy,
        },
      });
    } catch (error) {
      const problem = (error as { error?: ProblemDetails }).error;
      if (Number(problem?.status) === 409 && problem?.type === 'setup_already_complete') {
        setForm((current) => ({ ...current, password: '', confirmPassword: '' }));
        markSetupComplete();
        navigate('/login', { replace: true });
      }
      return;
    }

    markSetupComplete();
    if (response.data.nextStep === LoginNextStep.Completed && response.data.accessToken) {
      completeLogin(response.data.accessToken);
      const redirectTo = getPostLoginRedirect();
      navigate(redirectTo, { replace: true });
      return;
    }

    if (response.data.nextStep === LoginNextStep.EnrollMfa) {
      markMfaFlowStep('setup');
      navigate('/login/mfa/setup', { replace: true });
      return;
    }

    markMfaFlowStep('verify');
    navigate('/login/mfa', { replace: true });
  };

  return (
    <AuthShell title="Set up Citadel" description="Create your administrator account." panelClassName="max-w-md">
      {initialize.validationErrors && <AlertMessage type="warning">{initialize.validationErrors}</AlertMessage>}

      <form className="space-y-5" onSubmit={submit} noValidate aria-busy={initialize.isPending}>
        <fieldset className="min-w-0">
          <legend className="sr-only">Administrator account</legend>
          <div className="space-y-4">
            <Field
              id="setup-name"
              name="username"
              placeholder="e.g. admin"
              label="Username"
              value={form.name}
              error={errors.name}
              disabled={initialize.isPending}
              required
              autoComplete="username"
              autoCapitalize="none"
              spellCheck={false}
              onChange={(value) => updateField('name', value)}
            />
            <Field
              id="setup-email"
              name="email"
              placeholder="you@example.com"
              label="Email address"
              value={form.email}
              error={errors.email}
              disabled={initialize.isPending}
              required
              autoCapitalize="none"
              spellCheck={false}
              type="email"
              autoComplete="email"
              onChange={(value) => updateField('email', value)}
            />

            <div className="space-y-2">
              <label className="text-sm font-medium leading-none" htmlFor="setup-password">
                Password
              </label>
              <div className="relative">
                <Input
                  id="setup-password"
                  name="password"
                  required
                  type={showPassword ? 'text' : 'password'}
                  autoComplete="new-password"
                  value={form.password}
                  aria-invalid={Boolean(errors.password)}
                  aria-describedby="setup-password-description"
                  className="pr-11"
                  disabled={initialize.isPending}
                  onChange={(event) => updateField('password', event.target.value)}
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="absolute inset-y-0 right-0 size-10 text-muted-foreground hover:bg-transparent hover:text-foreground"
                  aria-label={showPassword ? 'Hide password' : 'Show password'}
                  aria-pressed={showPassword}
                  disabled={initialize.isPending}
                  onClick={() => setShowPassword((current) => !current)}>
                  {showPassword ? <EyeOff aria-hidden="true" /> : <Eye aria-hidden="true" />}
                </Button>
              </div>
              <p
                id="setup-password-description"
                role={errors.password ? 'alert' : undefined}
                className={errors.password ? 'text-xs font-medium text-destructive' : 'text-xs text-muted-foreground'}>
                {errors.password || `Use ${passwordMinimumLength} to ${passwordMaximumLength} characters.`}
              </p>
            </div>

            <Field
              id="setup-confirm-password"
              name="confirmPassword"
              required
              label="Confirm password"
              value={form.confirmPassword}
              error={errors.confirmPassword}
              disabled={initialize.isPending}
              type={showPassword ? 'text' : 'password'}
              autoComplete="new-password"
              onChange={(value) => updateField('confirmPassword', value)}
            />
          </div>
        </fieldset>

        <Collapsible className="border-y" defaultOpen={false}>
          <CollapsibleTrigger
            type="button"
            disabled={initialize.isPending}
            className="group flex min-h-12 w-full items-center justify-between gap-3 py-3 text-left text-sm font-medium text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset disabled:cursor-not-allowed disabled:opacity-50">
            Advanced
            <ChevronDown aria-hidden="true" className="size-4 shrink-0 group-data-[state=open]:rotate-180" />
          </CollapsibleTrigger>
          <CollapsibleContent>
            <div className="space-y-5 pb-5 pt-1">
              <div className="space-y-2">
                <div className="min-w-0 space-y-1">
                  <label htmlFor="setup-update-checks" className="text-sm font-medium">
                    Check for Citadel updates
                  </label>
                  <p id="setup-update-checks-description" className="text-xs leading-relaxed text-muted-foreground">
                    {updateChecksAllowed
                      ? 'Checks GitHub for new releases. No telemetry or automatic installation.'
                      : 'Disabled by server configuration.'}
                  </p>
                </div>
                <FieldSwitch
                  id="setup-update-checks"
                  checked={checkForUpdates}
                  disabled={initialize.isPending || !updateChecksAllowed}
                  aria-label="Check for Citadel updates"
                  aria-describedby="setup-update-checks-description"
                  onChange={setCheckForUpdates}
                />
              </div>
              <div className="space-y-2">
                <div className="space-y-2">
                  <label htmlFor="setup-mfa-policy" className="text-sm font-medium">
                    Two-factor authentication
                  </label>
                  <Select
                    value={mfaPolicy}
                    onValueChange={(value) => setMfaPolicy(value as MfaPolicy)}
                    disabled={initialize.isPending || minimumMfaPolicy === MfaPolicy.RequiredForAllUsers}>
                    <SelectTrigger
                      id="setup-mfa-policy"
                      className="h-10 w-full"
                      aria-describedby="setup-mfa-description">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {mfaPolicyOptions(minimumMfaPolicy).map((option) => (
                        <SelectItem key={option.value} value={option.value}>
                          {option.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <div className="space-y-1">
                  <p id="setup-mfa-description" className="text-xs leading-relaxed text-muted-foreground">
                    {mfaPolicy === MfaPolicy.Optional
                      ? 'MFA is optional. Set it up later in Profile → Security.'
                      : mfaPolicyDescription(mfaPolicy)}
                  </p>
                  {mfaPolicy !== MfaPolicy.Optional && (
                    <p className="text-xs leading-relaxed text-muted-foreground">
                      You’ll enroll after creating the administrator account.
                    </p>
                  )}
                </div>
                {minimumMfaPolicy !== MfaPolicy.Optional && (
                  <p className="text-xs leading-relaxed text-muted-foreground">
                    The minimum policy is set by server configuration.
                  </p>
                )}
              </div>
            </div>
          </CollapsibleContent>
        </Collapsible>

        <Button type="submit" className="h-10 w-full" disabled={initialize.isPending}>
          {initialize.isPending && <LoaderCircle aria-hidden="true" className="motion-safe:animate-spin" />}
          {initialize.isPending ? 'Creating administrator…' : 'Create administrator'}
        </Button>
      </form>
    </AuthShell>
  );
}

function Field({
  id,
  label,
  value,
  error,
  onChange,
  ...inputProps
}: {
  id: string;
  label: string;
  value: string;
  error?: string;
  onChange: (value: string) => void;
} & Omit<React.ComponentProps<typeof Input>, 'id' | 'value' | 'onChange'>) {
  return (
    <div className="space-y-2">
      <label className="text-sm font-medium leading-none" htmlFor={id}>
        {label}
      </label>
      <Input
        id={id}
        value={value}
        aria-invalid={Boolean(error)}
        aria-describedby={error ? `${id}-error` : undefined}
        onChange={(event) => onChange(event.target.value)}
        {...inputProps}
      />
      {error && (
        <p id={`${id}-error`} role="alert" className="text-xs font-medium text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}
