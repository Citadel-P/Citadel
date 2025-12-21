import { useState, FormEvent } from 'react';
import LogoIcon from '@/assets/logo.svg';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { LoaderCircle } from 'lucide-react';
import { AlertMessage } from '@/components/custom/alert-message';
import { useAuthContext } from './auth-context';

const EMAIL_REGEX = /^[a-zA-Z0-9._-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,6}$/;

const Login = () => {
  const { login, isPending, validationErrors } = useAuthContext();

  const [formData, setFormData] = useState({ email: '', password: '' });

  const [errors, setErrors] = useState<{ email?: string; password?: string }>({});

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    setFormData((prev) => ({ ...prev, [name]: value }));

    if (errors[name as keyof typeof errors]) {
      setErrors((prev) => ({ ...prev, [name]: undefined }));
    }
  };

  const validateForm = () => {
    const newErrors: { email?: string; password?: string } = {};
    let isValid = true;

    if (!formData.email) {
      newErrors.email = 'Email is required';
      isValid = false;
    } else if (!EMAIL_REGEX.test(formData.email)) {
      newErrors.email = 'Please enter a valid email address';
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

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();

    if (validateForm()) {
      login(formData);
    }
  };

  return (
    <div className="h-screen w-full overflow-hidden bg-card">
      <div className="mx-auto flex flex-col items-center justify-center px-6 py-8 md:h-screen lg:py-0">
        <div className="flex items-start mb-6">
          <span className="p-2 mr-2 w-9 h-9 text-background rounded bg-primary">
            <LogoIcon />
          </span>
          <span className="text-2xl font-semibold">Citadel</span>
        </div>

        <div className="w-full rounded-lg bg-background shadow-sm sm:max-w-md md:mt-0 xl:p-0">
          <div className="space-y-4 p-6 sm:p-8 md:space-y-6">
            <h1 className="text-xl font-bold leading-tight tracking-tight md:text-2xl">Sign in to your account</h1>

            {validationErrors && <AlertMessage type="warning">{validationErrors}</AlertMessage>}

            <form onSubmit={handleSubmit} className="space-y-8" noValidate>
              {/* Email Field */}
              <div className="space-y-2">
                <label htmlFor="email" className="text-sm font-medium leading-none">
                  Email address
                </label>
                <Input
                  id="email"
                  name="email"
                  type="email"
                  placeholder="Enter your email"
                  // Add conditional border color if error exists
                  className={`rounded-sm focus-visible:ring-transparent ${errors.email ? 'border-destructive' : ''}`}
                  value={formData.email}
                  onChange={handleChange}
                  disabled={isPending}
                />
                {errors.email && <p className="text-xs font-medium text-destructive">{errors.email}</p>}
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
          </div>
        </div>
      </div>
    </div>
  );
};

export default Login;
