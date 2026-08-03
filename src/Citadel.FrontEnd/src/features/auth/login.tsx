import { useMemo, useState, FormEvent } from 'react';
import LogoIcon from '@/assets/logo.svg';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { LoaderCircle, ShieldCheck } from 'lucide-react';
import { AlertMessage } from '@/components/custom/alert-message';
import { useAuthContext } from './auth-context';
import { Constants } from '@/lib/constants';
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

    if (!formData.emailOrName) {
      newErrors.emailOrName = 'Email address or username is required';
      isValid = false;
    } else if (!new RegExp(Constants.validEmailOrName).test(formData.emailOrName)) {
      newErrors.emailOrName = 'Please enter a valid email address or username';
      isValid = false;
    }

    if (!formData.password) {
      newErrors.password = 'Password is required';
      isValid = false;
    } else if (formData.password.length < 6) {
      newErrors.password = 'Password must be at least 6 characters';
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
    <div className="h-screen w-full overflow-hidden bg-card">
      <div className="mx-auto flex flex-col items-center justify-center px-6 py-8 md:h-screen lg:py-0">
        <div className="mb-6 flex items-center">
          <span className="mr-3 flex size-12 shrink-0 items-center justify-center">
            <LogoIcon className="size-12" />
          </span>
          <span className="text-2xl font-semibold">Citadel</span>
        </div>

        <div className="w-full rounded-lg bg-background shadow-sm sm:max-w-md md:mt-0 xl:p-0">
          <div className="space-y-4 p-6 sm:p-8 md:space-y-6">
            <h1 className="text-xl font-bold leading-tight tracking-tight md:text-2xl">Sign in to your account</h1>

            {validationErrors && <AlertMessage type="warning">{validationErrors}</AlertMessage>}

            <form onSubmit={handleSubmit} className="space-y-8" noValidate>
              {/* emailOrName Field */}
              <div className="space-y-2">
                <label htmlFor="emailOrName" className="text-sm font-medium leading-none">
                  Email address or username
                </label>
                <Input
                  id="emailOrName"
                  name="emailOrName"
                  type="text"
                  placeholder="Enter your email address or username"
                  className={`rounded-sm focus-visible:ring-transparent ${errors.emailOrName ? 'border-destructive' : ''}`}
                  value={formData.emailOrName}
                  onChange={handleChange}
                  disabled={isPending}
                />
                {errors.emailOrName && <p className="text-xs font-medium text-destructive">{errors.emailOrName}</p>}
              </div>

              {/* Password Field */}
              <div className="space-y-2">
                <label htmlFor="password" className="text-sm font-medium leading-none">
                  Password
                </label>
                <Input
                  id="password"
                  name="password"
                  type="password"
                  placeholder="Enter your password"
                  className={`rounded-sm focus-visible:ring-transparent ${errors.password ? 'border-destructive' : ''}`}
                  value={formData.password}
                  onChange={handleChange}
                  disabled={isPending}
                />
                {errors.password && <p className="text-xs font-medium text-destructive">{errors.password}</p>}
              </div>

              <Button
                type="submit"
                disabled={isPending}
                className="w-full rounded-lg bg-primary px-5 py-2.5 text-center text-sm font-medium text-background outline-hidden focus:ring-4 focus:ring-primary-300">
                Sign in
                {isPending && <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />}
              </Button>
            </form>

            {oidcProviders.length > 0 && (
              <div className="space-y-3">
                <div className="flex items-center gap-3">
                  <div className="h-px flex-1 bg-border" />
                  <span className="text-xs font-medium text-muted-foreground">or</span>
                  <div className="h-px flex-1 bg-border" />
                </div>

                <div className="space-y-2">
                  {oidcProviders.map((provider) => (
                    <Button
                      key={provider.id}
                      type="button"
                      variant="outline"
                      className="w-full"
                      disabled={isLoadingOidcProviders}
                      onClick={() => beginOidcLogin(provider.id)}>
                      <ShieldCheck />
                      Continue with {provider.displayName}
                    </Button>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default Login;
