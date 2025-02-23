import { Button } from '@/components/ui/button';
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { useAddDockerForm } from './useAddDockerForm';
import { PutPlatformRequest } from '@/api/_generated';
import { usePUTDockerPlatform } from './usePUTDockerPlatform';
import { Message } from '@/components/ui/message';
import { LoaderCircle } from 'lucide-react';

const ConnectAgentForm = () => {
  const { form } = useAddDockerForm();
  const { mutate, isPending, isSuccess, data, validationErrors } = usePUTDockerPlatform();

  function onSubmit(values: PutPlatformRequest) {
    mutate(values);
  }

  return (
    <Form {...form}>
      {isSuccess && (
        <Message type="success">
          <span>The platform {data?.data.name} has been added successfully</span>
        </Message>
      )}
      {validationErrors && <Message type="warning">{validationErrors}</Message>}
      <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-8">
        <FormField
          control={form.control}
          name="name"
          render={({ field }) => (
            <FormItem className="flex items-baseline">
              <FormLabel className="flex-none w-28 text-xs">Name</FormLabel>
              <div className="flex-1">
                <FormControl>
                  <Input
                    type="text"
                    className="rounded-sm focus-visible:ring-transparent"
                    placeholder="e.g. Platform-01"
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
          name="address"
          render={({ field }) => (
            <FormItem className="flex items-baseline">
              <FormLabel className="flex-none w-28 text-xs">Platform address</FormLabel>
              <div className="flex-1">
                <FormControl>
                  <Input
                    type="text"
                    className="rounded-sm focus-visible:ring-transparent"
                    placeholder="e.g. 172.25.192.1:8001 or myhostname:8001"
                    {...field}
                  />
                </FormControl>
                <FormMessage className="text-xs" />
              </div>
            </FormItem>
          )}
        />
        <Button type="submit" disabled={isPending}>
          <span>Connect</span>
          {isPending && <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />}
        </Button>
      </form>
    </Form>
  );
};

export default ConnectAgentForm;
