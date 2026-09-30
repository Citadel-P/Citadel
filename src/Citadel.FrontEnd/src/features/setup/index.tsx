import { FormEvent, useState } from 'react';
import { useNavigate } from 'react-router';
import { Eye, EyeOff, LoaderCircle } from 'lucide-react';
import { LoginNextStep, ProblemDetails } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { getPostLoginRedirect } from '@/features/auth/auth-route-guards';
import { useAuthContext } from '@/features/auth/auth-context';
import { MfaAuthShell } from '@/features/auth/mfa/components';
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
  const { markSetupComplete, passwordMinimumLength, passwordMaximumLength } = useSetupContext();
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
    <MfaAuthShell title="Set up Citadel" description="Create the administrator account for this installation.">
      {initialize.validationErrors && <AlertMessage type="warning">{initialize.validationErrors}</AlertMessage>}

      <form className="space-y-5" onSubmit={submit} noValidate>
        <Field
          id="setup-name"
          label="Username"
          value={form.name}
          error={errors.name}
          disabled={initialize.isPending}
          autoComplete="username"
          onChange={(value) => updateField('name', value)}
        />
        <Field
          id="setup-email"
          label="Email address"
          value={form.email}
          error={errors.email}
          disabled={initialize.isPending}
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
              type={showPassword ? 'text' : 'password'}
              autoComplete="new-password"
              value={form.password}
              aria-invalid={Boolean(errors.password)}
              className="pr-10"
              disabled={initialize.isPending}
              onChange={(event) => updateField('password', event.target.value)}
            />
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              className="absolute right-1 top-1"
              aria-label={showPassword ? 'Hide password' : 'Show password'}
              disabled={initialize.isPending}
              onClick={() => setShowPassword((current) => !current)}>
              {showPassword ? <EyeOff /> : <Eye />}
            </Button>
          </div>
          {errors.password ? (
            <p className="text-xs font-medium text-destructive">{errors.password}</p>
          ) : (
            <p className="text-xs text-muted-foreground">
              Use {passwordMinimumLength} to {passwordMaximumLength} characters.
            </p>
          )}
        </div>

        <Field
          id="setup-confirm-password"
          label="Confirm password"
          value={form.confirmPassword}
          error={errors.confirmPassword}
          disabled={initialize.isPending}
          type={showPassword ? 'text' : 'password'}
          autoComplete="new-password"
          onChange={(value) => updateField('confirmPassword', value)}
        />

        <Button type="submit" className="w-full" disabled={initialize.isPending}>
          Create administrator
          {initialize.isPending && <LoaderCircle className="animate-spin" />}
        </Button>
      </form>
    </MfaAuthShell>
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
        onChange={(event) => onChange(event.target.value)}
        {...inputProps}
      />
      {error && <p className="text-xs font-medium text-destructive">{error}</p>}
    </div>
  );
}
