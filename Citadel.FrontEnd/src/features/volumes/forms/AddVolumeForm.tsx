import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { CreateVolumeInput } from '@/api/_generated';
import { AlertMessage } from '@/components/ui/alert-message';
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useFieldArray } from 'react-hook-form';
import { LoaderCircle, ChevronDown } from 'lucide-react';
import KeyValuePairInput from '@/components/ui/KeyValuePairInput';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import React, { useEffect } from 'react';
import { useAppContext } from '@/AppProvider';
import { toast } from 'sonner';
import { useNavigate } from 'react-router';
import { useVolumeForm } from './hooks/useVolumeForm';
import { usePOSTVolume } from './hooks/usePOSTVolume';

const driverOptions = [{ value: 'local', label: 'Local' }];

const AddVolumeForm = () => {
  const { form } = useVolumeForm();
  const navigate = useNavigate();
  const { mutate, isPending, isSuccess, data, validationErrors } = usePOSTVolume();
  const { currentPlatform } = useAppContext();

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
      toast.success(`A new volume has been added successfully, volume ID: ${data?.data.id}`);
      navigate(`/platforms/${currentPlatform?.id}/volumes`);
    }
  }, [isSuccess, data, navigate, currentPlatform]);

  function onSubmit(values: CreateVolumeInput | Partial<CreateVolumeInput>) {
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
          <h5 className="text-md font-bold text-foreground">Create Volume</h5>
        </div>
        <ol className="relative border-s border-border ml-1">
          <Form {...form}>
            <form onSubmit={form.handleSubmit(onSubmit)}>
              {/* Basic Configuration */}
              <li className="mb-10 ms-6">
                {validationErrors && <AlertMessage type="error">{validationErrors}</AlertMessage>}
                <span className="absolute flex items-center justify-center w-5 h-5 bg-primary/80 rounded-full -start-2.5 ring-4 ring-background text-xs text-primary-foreground">
                  1
                </span>
                <h2 className="text-sm mb-2 font-semibold text-foreground">Basic Configuration</h2>
                <div className="space-y-4">
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
                              placeholder="e.g. my-volume"
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
                    name="driver"
                    render={({ field }) => (
                      <FormItem className="flex items-baseline">
                        <FormLabel className="flex-none w-36 text-xs">Driver</FormLabel>
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
              </li>

              {/* Advanced Configuration */}
              <li className="mb-10 ms-6">
                <span className="absolute flex items-center justify-center w-5 h-5 bg-primary/80 rounded-full -start-2.5 mt-1.5 ring-4 ring-background text-xs text-primary-foreground">
                  2
                </span>
                <Collapsible open={advancedOpen} onOpenChange={setAdvancedOpen}>
                  <CollapsibleTrigger asChild>
                    <button
                      type="button"
                      aria-expanded={advancedOpen}
                      className="flex items-center justify-between w-full px-2 py-2 rounded transition-colors hover:bg-accent group focus:outline-none">
                      <span className="text-sm font-semibold text-foreground group-hover:text-primary transition-colors">
                        Advanced Configuration (Optional)
                      </span>
                      <ChevronDown
                        className={`transition-transform ml-2 ${advancedOpen ? 'rotate-180' : ''} group-hover:text-primary`}
                        size={18}
                      />
                    </button>
                  </CollapsibleTrigger>
                  <CollapsibleContent>
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mt-4">
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
                        keyPlaceHolder="type"
                        valuePlaceHolder="nfs"
                        helpText="Handles how the volume's storage is managed on the underlying system or a remote storage provider, such as NFS, CIFS, or cloud services."
                      />
                    </div>
                  </CollapsibleContent>
                </Collapsible>
                <Button type="submit" className="mt-4" disabled={!form.formState.isDirty || !form.formState.isValid}>
                  <span>Create Volume</span>
                  {isPending && <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />}
                </Button>
              </li>
            </form>
          </Form>
        </ol>
      </div>
    </div>
  );
};

export default AddVolumeForm;
