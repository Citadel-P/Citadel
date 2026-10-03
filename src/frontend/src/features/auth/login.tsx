import { useMemo, useState, FormEvent } from 'react';
import { AuthShell } from './auth-shell';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Eye, EyeOff, LoaderCircle, ShieldCheck } from 'lucide-react';
import { AlertMessage } from '@/components/custom/alert-message';
import { useAuthContext } from './auth-context';
import { useRead } from '@/lib/hooks';
import { useApiClientContext } from '@/api/api-client-context';
import { LoginNextStep } from '@/api/generated/api.types';
import { useNavigate } from 'react-router';
import { clearMfaFlowStep, markMfaFlowStep } from './mfa/mfa-flow';

const Login = () => {
  const { login, isPending, validationErrors } = useAuthContext();
  const { apiClient } = useApiClientContext();
  const navigate = useNavigate();
  const { data: oidcProvidersData, isLoading: isLoadingOidcProviders } = useRead('listOidcLoginProviders');

  const [showPassword, setShowPassword] = useState(false);
  const [formData, setFormData] = useState({ emailOrName: '', password: '' });

  const [errors, setErrors] = useState<{ emailOrName?: string; password?: string }>({});
  const oidcProviders = oidcProvidersData?.data.providers ?? [];
  const apiBaseUrl = useMemo(() => (apiClient.baseUrl ?? '').replace(/\/+$/, ''), [apiClient.baseUrl]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    setFormData((prev) => ({ ...prev, [name]: value }));

    if (errors[name as keyof typeof errors]) {
      setErrors((prev) => ({ ...prev, [name]: undefined }));
    }
  };

  const validateForm = () => {
    const newErrors: { emailOrName?: string; password?: string } = {};
    let isValid = true;

    if (!formData.emailOrName.trim()) {
      newErrors.emailOrName = 'Email address or username is required';
      isValid = false;
    }

    if (!formData.password) {
      newErrors.password = 'Password is required';
      isValid = false;
    }

    setErrors(newErrors);
    return isValid;
  };

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();

    if (validateForm()) {
      clearMfaFlowStep();
      const response = await login(formData);
      if (response?.nextStep === LoginNextStep.VerifyMfa) {
        markMfaFlowStep('verify');
        navigate('/login/mfa');
      } else if (response?.nextStep === LoginNextStep.EnrollMfa) {
        markMfaFlowStep('setup');
        navigate('/login/mfa/setup');
      } else if (response?.nextStep === LoginNextStep.Completed) {
        clearMfaFlowStep();
      }
    }
  };

  const beginOidcLogin = (providerId: string) => {
    clearMfaFlowStep();
    const returnUrl = `${window.location.origin}/login`;
    window.location.assign(
      `${apiBaseUrl}/api/v1/authentication/oidc/${providerId}/login?returnUrl=${encodeURIComponent(returnUrl)}`,
    );
  };

  return (
    <AuthShell title="Sign in" description="Manage your containers and infrastructure.">
      {validationErrors && (
        <div role="alert">
          <AlertMessage type="warning">{validationErrors}</AlertMessage>
        </div>
      )}
      <form onSubmit={handleSubmit} className="space-y-5" noValidate aria-busy={isPending}>
        <div className="space-y-2">
          <label htmlFor="emailOrName" className="text-sm font-medium">
            Email address or username
          </label>
          <Input
            id="emailOrName"
            name="emailOrName"
            type="text"
            autoComplete="username"
            autoCapitalize="none"
            spellCheck={false}
            required
            placeholder="you@example.com"
            className="h-10"
            value={formData.emailOrName}
            onChange={handleChange}
            disabled={isPending}
            aria-invalid={!!errors.emailOrName}
            aria-describedby={errors.emailOrName ? 'login-identity-error' : undefined}
          />
          {errors.emailOrName && (
            <p id="login-identity-error" role="alert" className="text-xs text-destructive">
              {errors.emailOrName}
            </p>
          )}
        </div>
        <div className="space-y-2">
          <label htmlFor="password" className="text-sm font-medium">
            Password
          </label>
          <div className="relative">
            <Input
              id="password"
              name="password"
              type={showPassword ? 'text' : 'password'}
              autoComplete="current-password"
              required
              placeholder="Enter your password"
              className="h-10 pr-11"
              value={formData.password}
              onChange={handleChange}
              disabled={isPending}
              aria-invalid={!!errors.password}
              aria-describedby={errors.password ? 'login-password-error' : undefined}
            />
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="absolute inset-y-0 right-0 size-10 text-muted-foreground hover:bg-transparent hover:text-foreground"
              aria-label={showPassword ? 'Hide password' : 'Show password'}
              aria-pressed={showPassword}
              disabled={isPending}
              onClick={() => setShowPassword((value) => !value)}>
              {showPassword ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
            </Button>
          </div>
          {errors.password && (
            <p id="login-password-error" role="alert" className="text-xs text-destructive">
              {errors.password}
            </p>
          )}
        </div>
        <Button type="submit" disabled={isPending} className="h-10 w-full">
          {isPending && <LoaderCircle aria-hidden="true" className="size-4 motion-safe:animate-spin" />}
          {isPending ? 'Signing in…' : 'Sign in'}
        </Button>
      </form>
      {oidcProviders.length > 0 && (
        <div className="space-y-4">
          <div className="flex items-center gap-3" aria-hidden="true">
            <div className="h-px flex-1 bg-border" />
            <span className="text-xs text-muted-foreground">or</span>
            <div className="h-px flex-1 bg-border" />
          </div>
          <div className="space-y-2">
            {oidcProviders.map((provider) => (
              <Button
                key={provider.id}
                type="button"
                variant="outline"
                className="min-h-10 w-full whitespace-normal"
                disabled={isLoadingOidcProviders || isPending}
                onClick={() => beginOidcLogin(provider.id)}>
                <ShieldCheck aria-hidden="true" className="size-4 shrink-0" />
                Continue with {provider.displayName}
              </Button>
            ))}
          </div>
        </div>
      )}
    </AuthShell>
  );
};

export default Login;
