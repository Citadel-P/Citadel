import { AlertMessage } from '@/components/custom/alert-message';
import { useDockerHubForm } from './hooks/use-dockerhub-form';
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { LoaderCircle } from 'lucide-react';
import { useRegistryFormContext } from './registry-form-context';
import { FieldChange } from '@/components/ui/field-change';
import { RegistryInput } from '@/api/generated/api.types';
import { getEditedFields } from '@/lib/utils';

const DockerHubConfiguration = () => {
  const { form } = useDockerHubForm();
  const { saveButtonTitle, onPostForm, isLoading, isLoadingForm, validationErrors, mode } = useRegistryFormContext();

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
        To create a DockerHub Access Token, please follow the steps in the official{' '}
        <a
          className="underline"
          target="_blank"
          rel="noreferrer"
          href="https://docs.docker.com/security/for-developers/access-tokens/">
          DockerHub documentation
        </a>
        .
      </AlertMessage>

      <Form {...form}>
        {validationErrors && <AlertMessage type="error">{validationErrors}</AlertMessage>}
        <form onSubmit={form.handleSubmit(onSubmit as any)} className="space-y-6">
          <FormField
            control={form.control}
            name="name"
            render={({ field }) => (
              <FormItem className="flex flex-col sm:flex-row sm:items-baseline">
                <FormLabel className="flex-none w-full sm:w-36 text-xs mb-2 sm:mb-0">Name</FormLabel>
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
              <FormItem className="flex flex-col sm:flex-row sm:items-baseline">
                <FormLabel className="flex-none w-full sm:w-36 text-xs mb-2 sm:mb-0">Url</FormLabel>
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
              <FormItem className="flex flex-col sm:flex-row sm:items-baseline">
                <FormLabel className="flex-none w-full sm:w-36 text-xs mb-2 sm:mb-0">DockerHub Username</FormLabel>
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
              <FormItem className="flex flex-col sm:flex-row sm:items-baseline">
                <FormLabel className="flex-none w-full sm:w-36 text-xs mb-2 sm:mb-0">DockerHub PAT</FormLabel>
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
          <Button
            type="submit"
            disabled={isLoading || isLoadingForm || !form.formState.isDirty || !form.formState.isValid}>
            <span>{saveButtonTitle}</span>
            {(isLoading || isLoadingForm) && <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />}
          </Button>
        </form>
      </Form>
    </div>
  );
};

export default DockerHubConfiguration;
