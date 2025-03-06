import { AlertMessage } from '@/components/ui/alert-message';
import { useGhcrForm } from './hooks/useGhcrForm';
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { LoaderCircle } from 'lucide-react';
import { usePOSTRegistry } from './hooks/usePOSTRegistry';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { useNavigate } from 'react-router';
import { CreateRegistryInput } from '@/api/_generated';

const GhcrConfiguration = () => {
  const { form } = useGhcrForm();
  const navigate = useNavigate();
  const [accountType, setAccountType] = useState('Organization');
  const { mutate, validationErrors, isSuccess, isPending, data } = usePOSTRegistry();

  const onSubmit = (values: CreateRegistryInput) => {
    mutate(values);
  };

  useEffect(() => {
    if (isSuccess && data?.data) {
      toast.success(`The ${data?.data.name} registry has been added`);
      navigate('/registries');
    }
  }, [isSuccess, data, navigate]);

  return (
    <div>
      <AlertMessage type="info">
        Please provide a Personal Access Token with `read-package` scope, follow the{' '}
        <a
          className="hover:underline"
          target="_blank"
          rel="noreferrer"
          href="https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens#creating-a-personal-access-token-classic">
          GitHub guide
        </a>
        .
      </AlertMessage>

      <Form {...form}>
        {validationErrors && <AlertMessage type="error">{validationErrors}</AlertMessage>}
        <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-8">
          <FormField
            control={form.control}
            name="name"
            render={({ field }) => (
              <FormItem className="flex items-baseline">
                <FormLabel className="flex-none w-36 text-xs">Name</FormLabel>
                <div className="flex-1">
                  <FormControl>
                    <Input
                      type="text"
                      placeholder="my-ghcr-registry"
                      className="rounded-sm focus-visible:ring-transparent"
                      {...field}
                    />
                  </FormControl>
                  <FormMessage className="text-xs" />
                </div>
              </FormItem>
            )}
          />
          <FormField
            control={form.control}
            name="url"
            render={({ field }) => (
              <FormItem className="flex items-baseline">
                <FormLabel className="flex-none w-36 text-xs">Url</FormLabel>
                <FormControl className="flex-1">
                  <Input
                    type="text"
                    placeholder="https://ghcr.io"
                    disabled
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
            name="configuration.type"
            render={({ field }) => (
              <FormItem className="flex items-baseline">
                <FormLabel className="flex-none w-36 text-xs">Account type</FormLabel>
                <Select
                  onValueChange={(v) => {
                    setAccountType(v);
                    field.onChange(v);
                  }}
                  defaultValue={field.value}>
                  <FormControl className="flex-1">
                    <SelectTrigger>
                      <SelectValue placeholder="Select your account type" />
                    </SelectTrigger>
                  </FormControl>
                  <SelectContent className="bg-background">
                    <SelectItem value="Organization">Organization</SelectItem>
                    <SelectItem value="User">User</SelectItem>
                  </SelectContent>
                </Select>
                <FormMessage />
              </FormItem>
            )}
          />

          <FormField
            control={form.control}
            name="configuration.name"
            render={({ field }) => (
              <FormItem className="flex items-baseline">
                <FormLabel className="flex-none w-36 text-xs">
                  {accountType === 'Organization' ? 'Organization name' : 'User name'}
                </FormLabel>
                <div className="flex-1">
                  <FormControl>
                    <Input
                      type="text"
                      placeholder=""
                      className="rounded-sm focus-visible:ring-transparent"
                      {...field}
                    />
                  </FormControl>
                  <FormMessage className="text-xs" />
                </div>
              </FormItem>
            )}
          />
          <FormField
            control={form.control}
            name="configuration.pat"
            render={({ field }) => (
              <FormItem className="flex items-baseline">
                <FormLabel className="flex-none w-36 text-xs">PAT</FormLabel>
                <div className="flex-1">
                  <FormControl>
                    <Input
                      type="text"
                      placeholder=""
                      className="rounded-sm focus-visible:ring-transparent"
                      {...field}
                    />
                  </FormControl>
                  <FormMessage className="text-xs" />
                </div>
              </FormItem>
            )}
          />
          <Button type="submit" className="dark:text-foreground " disabled={isPending}>
            <span>Add registry</span>
            {isPending && <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />}
          </Button>
        </form>
      </Form>
    </div>
  );
};

export default GhcrConfiguration;
