import { AlertMessage } from '@/components/ui/alert-message';
import { useDockerHubForm } from './hooks/useDockerHubForm';
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { LoaderCircle } from 'lucide-react';
import { useRegistryFormContext } from './RegistryFormProvider';
import { FieldChange } from '@/components/ui/field-change';
import { RegistryInput } from '@/api/_generated';
import { getEditedFields } from '@/lib/utils';

const DockerHubConfiguration = () => {
  const { form } = useDockerHubForm();
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
        For information on how to generate a DockerHub Access Token, follow the{' '}
        <a
          className="hover:underline"
          target="_blank"
          rel="noreferrer"
          href="https://docs.docker.com/security/for-developers/access-tokens/">
          DockerHub guide
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
                  {mode === 'edit' && <FieldChange form={form} fieldName={field.name} />}
                  <FormControl>
                    <Input
                      type="text"
                      placeholder="my-dockerhub-registry"
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
                    placeholder=""
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
            name="configuration.username"
            render={({ field }) => (
              <FormItem className="flex items-baseline">
                <FormLabel className="flex-none w-36 text-xs">DockerHub Username</FormLabel>
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
                <FormLabel className="flex-none w-36 text-xs">DockerHub PAT</FormLabel>
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
          <Button type="submit" disabled={isLoading || !form.formState.isDirty}>
            <span>{saveButtonTitle}</span>
            {isLoading && <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />}
          </Button>
        </form>
      </Form>
    </div>
  );
};

export default DockerHubConfiguration;
