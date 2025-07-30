import { AlertMessage } from '@/components/ui/alert-message';
import { useGhcrForm } from './hooks/useGhcrForm';
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { LoaderCircle } from 'lucide-react';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useRegistryFormContext } from './RegistryFormContext';
import { FieldChange } from '@/components/ui/field-change';
import { RegistryInput } from '@/api/_generated';
import { getEditedFields } from '@/lib/utils';
import { useState } from 'react';
import { Badge } from '@/components/ui/badge';

const GhcrConfiguration = () => {
  const { form } = useGhcrForm();

  const [accountType, setAccountType] = useState('Organization');
  const { saveButtonTitle, onPostForm, isLoading, validationErrors, mode } = useRegistryFormContext();

  function onSubmit(values: RegistryInput | Partial<RegistryInput>) {
    if (mode === 'edit') {
      const dirtyFields = form.formState.dirtyFields;
      values = getEditedFields(dirtyFields, values);
    }
    onPostForm!(values);
  }
  return (
    <div>
      <AlertMessage type="info">
        Please provide a Personal Access Token (PAT) with the <Badge variant="secondary">read:packages</Badge> scope to
        proceed. You can follow the{' '}
        <a
          className="underline"
          target="_blank"
          rel="noreferrer"
          href="https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens#creating-a-personal-access-token-classic">
          GitHub guide
        </a>{' '}
        to generate one.
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
                  {mode === 'edit' && <FieldChange form={form} fieldName={field.name} />}
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
                <div className="flex-1">
                  <Select
                    onValueChange={(v) => {
                      setAccountType(v);
                      field.onChange(v);
                    }}
                    {...field}>
                    {mode === 'edit' && <FieldChange form={form} fieldName={field.name} />}
                    <FormControl className="w-full shadow-none">
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
                </div>
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
                  {mode === 'edit' && <FieldChange form={form} fieldName={field.name} />}
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
                  {mode === 'edit' && <FieldChange form={form} fieldName={field.name} />}
                  <FormControl>
                    <Input
                      type="password"
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
          <Button type="submit" disabled={isLoading || !form.formState.isDirty || !form.formState.isValid}>
            <span>{saveButtonTitle}</span>
            {isLoading && <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />}
          </Button>
        </form>
      </Form>
    </div>
  );
};

export default GhcrConfiguration;
