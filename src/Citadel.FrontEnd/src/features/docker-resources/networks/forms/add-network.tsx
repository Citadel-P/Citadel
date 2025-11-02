import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { Button } from '@/components/ui/button';
import { useNetworkForm } from './hooks/useNetworkForm';
import { CreateNetworkInput } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { Form, FormControl, FormDescription, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useFieldArray } from 'react-hook-form';
import { LoaderCircle, ChevronDown } from 'lucide-react';
import KeyValuePairInput from '@/components/custom/key-value-pair-input';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import React, { useEffect } from 'react';
import { useAppContext } from '@/lib/context/app-context';
import { toast } from 'sonner';
import { useNavigate } from 'react-router';
import { useMutate } from '@/lib/hooks';

const driverOptions = [
  { value: 'bridge', label: 'Bridge' },
  { value: 'overlay', label: 'Overlay' },
  { value: 'ipvlan', label: 'Ipvlan' },
  { value: 'macvlan', label: 'Macvlan' },
];

function IPField({
  control,
  name,
  label,
  placeholder,
}: {
  control: any;
  name: string;
  label: string;
  placeholder: string;
}) {
  return (
    <FormField
      control={control}
      name={name}
      render={({ field }) => (
        <FormItem className="flex flex-col sm:flex-row sm:items-baseline">
          <FormLabel className="flex-none w-full sm:w-36 font-normal text-sm mb-2 sm:mb-0">{label}</FormLabel>
          <div className="flex-1">
            <FormControl>
              <Input
                type="text"
                placeholder={placeholder}
                className="rounded-sm focus-visible:ring-transparent"
                {...field}
              />
            </FormControl>
            <FormMessage className="text-sm" />
          </div>
        </FormItem>
      )}
    />
  );
}

export default function AddNetwork() {
  const { form } = useNetworkForm();
  const navigate = useNavigate();
  const { mutate, isPending, isSuccess, data, validationErrors } = useMutate('createNetwork');
  const { currentPlatform } = useAppContext();

  const enableIPv4 = form.watch('enableIPv4');
  const enableIPv6 = form.watch('enableIPv6');
  const { control } = form;
  const [advancedOpen, setAdvancedOpen] = React.useState(false);

  // For options
  const {
    fields: optionFields,
    append: appendOption,
    remove: removeOption,
  } = useFieldArray({
    control,
    name: 'options',
  });

  // For labels
  const {
    fields: labelFields,
    append: appendLabel,
    remove: removeLabel,
  } = useFieldArray({
    control,
    name: 'labels',
  });

  useEffect(() => {
    if (isSuccess && data?.data) {
      toast.success(`A new network has been added successfully, network ID: ${data?.data.id}`);
      navigate(`/platforms/${currentPlatform?.id}/networks`);
    }
  }, [isSuccess, data, navigate, currentPlatform]);

  function onSubmit(values: CreateNetworkInput | Partial<CreateNetworkInput>) {
    const optionsObj = Object.fromEntries((values.options ?? []).map(({ key, value }) => [key, value]));
    const labelsObj = Object.fromEntries((values.labels ?? []).map(({ key, value }) => [key, value]));

    values.platformId = currentPlatform?.id;
    values.options = optionsObj;
    values.labels = labelsObj;
    mutate(values);
  }

  return (
    <div className="mx-auto px-4 py-3 lg:container sm:px-6">
      <div className="w-full rounded-lg border border-border bg-background p-4">
        <div className="mb-4 flex items-center justify-between">
          <h5 className="text-md font-bold text-foreground">Create Network</h5>
        </div>
        <ol className="relative border-s border-border ml-1">
          <Form {...form}>
            <form onSubmit={form.handleSubmit(onSubmit)}>
              {/* Basic Configuration */}
              <li className="mb-10 ms-6">
                {validationErrors && <AlertMessage type="error">{validationErrors}</AlertMessage>}
                <span className="absolute flex items-center justify-center w-5 h-5 bg-primary/80 rounded-full -start-2.5 ring-4 ring-background text-sm text-primary-foreground">
                  1
                </span>
                <div className="flex flex-col space-y-4">
                  <div className="text-sm mt-0.5 font-semibold text-foreground leading-none">Basic Configuration</div>
                  <div className="space-y-4">
                    <FormField
                      control={form.control}
                      name="name"
                      render={({ field }) => (
                        <FormItem className="flex flex-col sm:flex-row sm:items-baseline">
                          <FormLabel className="flex-none w-full sm:w-36 font-normal text-sm mb-2 sm:mb-0">
                            Name
                          </FormLabel>
                          <div className="flex-1">
                            <FormControl>
                              <Input
                                type="text"
                                placeholder="e.g. my-network"
                                className="rounded-sm focus-visible:ring-transparent"
                                {...field}
                              />
                            </FormControl>
                            <FormMessage className="text-sm" />
                          </div>
                        </FormItem>
                      )}
                    />

                    <FormField
                      control={form.control}
                      name="driver"
                      render={({ field }) => (
                        <FormItem className="flex flex-col sm:flex-row sm:items-baseline">
                          <FormLabel className="flex-none w-full sm:w-36 font-normal text-sm mb-2 sm:mb-0">
                            Driver
                          </FormLabel>
                          <div className="flex-1">
                            <Select onValueChange={field.onChange} value={field.value}>
                              <FormControl className="w-full shadow-none">
                                <SelectTrigger>
                                  <SelectValue placeholder="Select your account type" />
                                </SelectTrigger>
                              </FormControl>
                              <SelectContent className="bg-background">
                                {driverOptions.map((option) => (
                                  <SelectItem key={option.value} value={option.value}>
                                    {option.label}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                            <FormMessage />
                          </div>
                        </FormItem>
                      )}
                    />
                  </div>
                </div>
              </li>

              {/* Advanced Configuration */}
              <li className="mb-10 ms-6">
                <span className="absolute flex items-center justify-center w-5 h-5 bg-primary/80 rounded-full -start-2.5 mt-1.5 ring-4 ring-background text-sm text-primary-foreground">
                  2
                </span>
                <Collapsible open={advancedOpen} onOpenChange={setAdvancedOpen}>
                  <div className="flex flex-col space-y-4">
                    <CollapsibleTrigger asChild>
                      <button
                        type="button"
                        aria-expanded={advancedOpen}
                        className="flex items-center justify-between w-full px-2 py-2 rounded transition-colors hover:bg-accent group focus:outline-none -mt-0.5">
                        <span className="text-sm font-semibold text-foreground leading-none group-hover:text-primary transition-colors">
                          Advanced Configuration (Optional)
                        </span>
                        <ChevronDown
                          className={`transition-transform ml-2 ${advancedOpen ? 'rotate-180' : ''} group-hover:text-primary`}
                          size={18}
                        />
                      </button>
                    </CollapsibleTrigger>
                    <CollapsibleContent>
                      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
                        <FormField
                          control={form.control}
                          name="enableIPv4"
                          render={({ field }) => (
                            <FormItem className="col-span-2 flex flex-row items-center justify-between rounded-lg border p-3">
                              <div className="space-y-0.5">
                                <FormLabel>Enable IPv4</FormLabel>
                                <FormDescription>
                                  <FormMessage className="text-sm" />
                                  Controls whether IPv4 address allocation is enabled for this network.
                                </FormDescription>
                              </div>
                              <FormControl>
                                <Switch checked={field.value} onCheckedChange={field.onChange} />
                              </FormControl>
                            </FormItem>
                          )}
                        />

                        {enableIPv4 && (
                          <>
                            <div className="col-span-2 md:col-span-1">
                              <IPField
                                control={form.control}
                                name="ipam.config.0.subnet"
                                label="IPv4 Subnet"
                                placeholder="e.g. 192.168.1.0/24"
                              />
                            </div>
                            <div className="col-span-2 md:col-span-1">
                              <IPField
                                control={form.control}
                                name="ipam.config.0.gateway"
                                label="IPv4 Gateway"
                                placeholder="e.g. 192.168.1.1"
                              />
                            </div>
                            <div className="col-span-2 md:col-span-1">
                              <IPField
                                control={form.control}
                                name="ipam.config.0.ipRange"
                                label="IPv4 Range"
                                placeholder="e.g. 192.168.1.0/25"
                              />
                            </div>
                          </>
                        )}

                        <FormField
                          control={form.control}
                          name="enableIPv6"
                          render={({ field }) => (
                            <FormItem className="col-span-2 flex flex-row items-center justify-between rounded-lg border p-3">
                              <div className="space-y-0.5">
                                <FormLabel>Enable IPv6</FormLabel>
                                <FormDescription>
                                  Controls whether IPv6 address allocation is enabled for this network.
                                </FormDescription>
                              </div>
                              <FormControl>
                                <Switch checked={field.value} onCheckedChange={field.onChange} />
                              </FormControl>
                              <FormMessage className="text-sm" />
                            </FormItem>
                          )}
                        />
                        {enableIPv6 && (
                          <>
                            <div className="col-span-2 md:col-span-1">
                              <IPField
                                control={form.control}
                                name="ipam.config.1.subnet"
                                label="IPv6 Subnet"
                                placeholder="e.g. fd00::/64"
                              />
                            </div>
                            <div className="col-span-2 md:col-span-1">
                              <IPField
                                control={form.control}
                                name="ipam.config.1.gateway"
                                label="IPv6 Gateway"
                                placeholder="e.g. fd00::1"
                              />
                            </div>
                            <div className="col-span-2 md:col-span-1">
                              <IPField
                                control={form.control}
                                name="ipam.config.1.ipRange"
                                label="IPv6 Range"
                                placeholder="e.g. fd00::/64"
                              />
                            </div>
                          </>
                        )}
                        <KeyValuePairInput
                          name="labels"
                          fields={labelFields}
                          control={control}
                          append={appendLabel}
                          remove={removeLabel}
                          label="Labels"
                          addButtonLabel="Add label"
                          keyPlaceHolder="com.example.foo"
                          valuePlaceHolder="bar"
                        />
                        <KeyValuePairInput
                          name="options"
                          fields={optionFields}
                          control={control}
                          append={appendOption}
                          remove={removeOption}
                          label="Driver Options"
                          addButtonLabel="Add driver option"
                        />

                        <div className="col-span-2 md:col-span-1">
                          <FormField
                            control={form.control}
                            name="internal"
                            render={({ field }) => (
                              <FormItem className="flex flex-row items-center justify-between rounded-lg border p-3 h-full">
                                <div className="space-y-0.5">
                                  <FormLabel>Internal</FormLabel>
                                  <FormDescription>
                                    Restrict external access to and from this network. This feature provides network
                                    isolation for containers.
                                  </FormDescription>
                                </div>
                                <FormControl>
                                  <Switch checked={field.value} onCheckedChange={field.onChange} />
                                </FormControl>
                              </FormItem>
                            )}
                          />
                        </div>

                        <div className="col-span-2 md:col-span-1">
                          <FormField
                            control={form.control}
                            name="attachable"
                            render={({ field }) => (
                              <FormItem className="flex flex-row items-center justify-between rounded-lg border p-3 h-full">
                                <div className="space-y-0.5">
                                  <FormLabel>Attachable</FormLabel>
                                  <FormDescription>
                                    Controls which types of containers can connect to an overlay network.
                                  </FormDescription>
                                </div>
                                <FormControl>
                                  <Switch checked={field.value} onCheckedChange={field.onChange} />
                                </FormControl>
                              </FormItem>
                            )}
                          />
                        </div>
                      </div>
                    </CollapsibleContent>
                  </div>
                </Collapsible>
                <Button type="submit" className="mt-4" disabled={!form.formState.isDirty || !form.formState.isValid}>
                  <span>Create Network</span>
                  {isPending && <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />}
                </Button>
              </li>
            </form>
          </Form>
        </ol>
      </div>
    </div>
  );
}
