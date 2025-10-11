import LogoIcon from '@/assets/logo.svg';
import { Button } from '@/components/ui/button';
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { useLoginForm } from './hooks/useLoginForm';
import { LoginRequest } from '@/api/generated/api.types';
import { LoaderCircle } from 'lucide-react';
import { AlertMessage } from '@/components/ui/alert-message';
import { useEffect } from 'react';
import { ACCESS_TOKEN_KEY } from './AuthProvider';
import { useMutate } from '@/lib/hooks';

const Login = () => {
  const { form } = useLoginForm();
  const { mutate, isSuccess, data, isPending, validationErrors } = useMutate('login');

  useEffect(() => {
    if (isSuccess && data?.data) {
      sessionStorage.setItem(ACCESS_TOKEN_KEY, data?.data.accessToken);
      window.location.href = '/';
    }
  }, [isSuccess, data]);

  const onSubmit = (values: LoginRequest) => {
    mutate(values);
  };

  return (
    <div className=" h-screen w-full overflow-hidden bg-card">
      <div className="mx-auto flex flex-col items-center justify-center px-6 py-8 md:h-screen lg:py-0">
        <div className="flex items-start mb-6">
          <span className="p-2 mr-2 w-9 h-9 text-background rounded bg-primary ">
            <LogoIcon />
          </span>
          <span className="text-2xl font-semibold">Citadel</span>
        </div>

        <div className="w-full rounded-lg bg-background shadow-sm sm:max-w-md md:mt-0 xl:p-0">
          <div className="space-y-4 p-6 sm:p-8 md:space-y-6">
            <h1 className="text-xl font-bold leading-tight tracking-tight md:text-2xl">Sign in to your account</h1>
            {validationErrors && <AlertMessage type="warning">{validationErrors} </AlertMessage>}
            <Form {...form}>
              <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-8">
                <FormField
                  control={form.control}
                  name="email"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Email address</FormLabel>
                      <FormControl>
                        <Input
                          type="text"
                          placeholder="Enter your email"
                          className="rounded-sm focus-visible:ring-transparent"
                          {...field}
                        />
                      </FormControl>
                      <FormMessage className="text-xs" />
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="password"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Password</FormLabel>
                      <FormControl>
                        <Input
                          type="password"
                          placeholder="Enter your password"
                          className="rounded-sm focus-visible:ring-transparent"
                          {...field}
                        />
                      </FormControl>
                      <FormMessage className="text-xs" />
                    </FormItem>
                  )}
                />
                <Button
                  type="submit"
                  disabled={isPending}
                  className="w-full rounded-lg bg-primary px-5 py-2.5 text-center text-sm font-medium text-background outline-hidden focus:ring-4 focus:ring-primary-300">
                  Sign in
                  {isPending && <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />}
                </Button>
              </form>
            </Form>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Login;
