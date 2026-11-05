import { PasswordFeedback } from '@/components/custom/password-feedback';
import { FormEvent, useState } from 'react';
import { useNavigate } from 'react-router';
import { Eye, EyeOff, LoaderCircle } from 'lucide-react';
import { LoginNextStep, ProblemDetails } from '@/api/generated/api.types';
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

function confirmationError(password: string, confirmation: string) {
  if (!confirmation) return 'Confirm your password.';
  if (password !== confirmation) return 'Passwords do not match.';
  return undefined;
}

export default function InitialSetup() {
  const navigate = useNavigate();
  const { completeLogin } = useAuthContext();
  const {
    markSetupComplete,
    passwordMinimumLength,
    passwordMaximumLength,
    updateChecksEnabled,
    updateChecksAllowed,
    mfaPolicy,
  } = useSetupContext();
  const initialize = useMutate('initializeCitadel');
  const [form, setForm] = useState(initialForm);
  const [errors, setErrors] = useState<SetupErrors>({});
  const [showPassword, setShowPassword] = useState(false);

  const updateField = (field: keyof SetupForm, value: string) => {
    const next = { ...form, [field]: value };
    setForm(next);
    setErrors((current) => ({
      ...current,
      [field]: undefined,
      ...((field === 'password' || field === 'confirmPassword') && current.confirmPassword
        ? { confirmPassword: confirmationError(next.password, next.confirmPassword) }
        : {}),
    }));
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

    const confirmation = confirmationError(form.password, form.confirmPassword);
    if (confirmation) next.confirmPassword = confirmation;

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
          updateChecksEnabled: updateChecksEnabled && updateChecksAllowed,
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
                  aria-describedby={
                    errors.password ? 'setup-password-description setup-password-error' : 'setup-password-description'
                  }
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
              <PasswordFeedback
                id="setup-password-description"
                password={form.password}
                minimumLength={passwordMinimumLength}
                maximumLength={passwordMaximumLength}
              />
              {errors.password && (
                <p id="setup-password-error" role="alert" className="text-xs font-medium text-destructive">
                  {errors.password}
                </p>
              )}
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
