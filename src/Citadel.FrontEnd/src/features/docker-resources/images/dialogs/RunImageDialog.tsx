import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog';

import { useAppContext } from '@/lib/context/app-context';
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from '@/components/ui/collapsible';
import KeyValuePairInput from '@/components/custom/key-value-pair-input';
import { LoaderCircle, ChevronDown, Info } from 'lucide-react';
import { useFieldArray } from 'react-hook-form';
import { DialogFooter } from '@/components/ui/dialog';
import { useEffect, useState } from 'react';
import { useRunImageForm } from '../hooks/useRunImageForm';
import { Form, FormControl, FormField, FormItem, FormMessage, FormLabel } from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import PortMappingInput from '../components/PortMappingInput';
import VolumeMappingInput from '../components/VolumeMappingInput';
import { MultiSelect } from '@/components/ui/multi-select';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Slider } from '@/components/ui/slider';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { Switch } from '@/components/ui/switch';
import ValueInput from '@/components/custom/value-input';
import { ContainerRestartPolicy, CreateContainerInput, ImageView, ProblemDetails } from '@/api/generated/api.types';
import { toast } from 'sonner';
import { useNavigate } from 'react-router';
import { AlertMessage } from '@/components/custom/alert-message';
import { IDialogData, useMutate, useRead } from '@/lib/hooks';

export const RunImageDialog = ({
  runDialogData,
  setRunDialogData,
}: {
  runDialogData: IDialogData<ImageView>;
  setRunDialogData: (_: IDialogData<ImageView>) => void;
}) => {
  const navigate = useNavigate();
  const { currentPlatform } = useAppContext();
  const { mutate, isPending, isSuccess, data, validationErrors, reset } = useMutate('createContainer');
  const { form } = useRunImageForm();
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const {
    data: imageInfo,
    isLoading: imageInfoIsLoading,
    isSuccess: imageInfoIsSuccess,
    error: imageInfoError,
  } = useRead('getImageInfo', {
    platformId: currentPlatform?.id,
    imageId: runDialogData.currentSelection?.at(0)?.dockerImageId,
  });
  const { control } = form;

  useEffect(() => {
    if (imageInfoIsSuccess) {
      const exposedPorts = imageInfo?.data.exposedPorts;
      if (exposedPorts) {
        form.setValue('ports', exposedPorts);
      }
    }
  }, [imageInfo, imageInfoIsSuccess, form]);

  useEffect(() => {
    if (isSuccess && data?.data) {
      toast.success('Container created successfully.');

      const timer = setTimeout(() => {
        navigate(`/containers/${data.data.id.slice(0, 12)}/logs`);
      }, 1000);

      return () => clearTimeout(timer);
    }
  }, [isSuccess, data, navigate, currentPlatform]);

  // For env varaibles
  const {
    fields: envVarsFields,
    append: appendEnvVar,
    remove: removeEnvVar,
  } = useFieldArray({
    control,
    name: 'envVars',
  });
  // For volumes
  const {
    fields: volumesFields,
    append: appendVolume,
    remove: removeVolume,
  } = useFieldArray({
    control,
    name: 'volumes',
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

  const {
    fields: entryPointFields,
    append: appendEntryPoint,
    remove: removeEntryPoint,
  } = useFieldArray({
    control,
    name: 'entryPoint',
  });

  const {
    fields: commandFields,
    append: appendCommand,
    remove: removeCommand,
  } = useFieldArray({
    control,
    name: 'command',
  });

  function onSubmit(values: CreateContainerInput | Partial<CreateContainerInput>) {
    const finalValues = {
      ...values,
      volumes: values.volumes
        ?.filter((v: any) => v.hostPath && v.containerPath)
        .map((v: any) => `${v.hostPath}:${v.containerPath}`),
      entryPoint: values.entryPoint?.map((e: any) => e.value),
      command: values.command?.map((c: any) => c.value),
    };
    const envVarsArr = (values.envVars ?? []).map(({ key, value }) => `${key}=${value}`);
    const labelsObj = Object.fromEntries((values.labels ?? []).map(({ key, value }) => [key, value]));
    finalValues.envVars = envVarsArr;
    finalValues.labels = labelsObj;
    finalValues.platformId = currentPlatform?.id;
    finalValues.imageId = runDialogData.currentSelection?.at(0)?.dockerImageId;

    mutate(finalValues as any);
  }

  function onOpenChange(open: boolean) {
    if (!open) {
      form.reset();
      reset();
      setAdvancedOpen(false);
      labelFields.map((_, i) => removeLabel(i));
      envVarsFields.map((_, i) => removeEnvVar(i));
      volumesFields.map((_, i) => removeVolume(i));
      commandFields.map((_, i) => removeEntryPoint(i));
      entryPointFields.map((_, i) => removeEntryPoint(i));
    }
    setRunDialogData({ open });
  }

  const RESTART_POLICIES = [
    { value: ContainerRestartPolicy.No, label: 'No' },
    { value: ContainerRestartPolicy.Always, label: 'Always' },
    { value: ContainerRestartPolicy.UnlessStopped, label: 'Unless Stopped' },
    { value: ContainerRestartPolicy.OnFailure, label: 'On Failure' },
  ];

  return (
    <Dialog open={runDialogData.open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[700px]">
        {imageInfoError ? (
          <AlertMessage type="error">{((imageInfoError as any)?.error as ProblemDetails)?.detail}</AlertMessage>
        ) : (
          <Form {...form}>
            <form onSubmit={form.handleSubmit(onSubmit)}>
              <DialogHeader>
                <DialogTitle>Create a new container</DialogTitle>
                <DialogDescription>
                  {runDialogData.currentSelection?.at(0)?.name ?? '-'}:
                  {runDialogData.currentSelection?.at(0)?.tag ?? '-'}
                </DialogDescription>
              </DialogHeader>
              <div className="grid gap-4 py-4">
                <Collapsible open={advancedOpen} onOpenChange={setAdvancedOpen}>
                  <div className="flex flex-col space-y-2 border-t-1 border-b-1 border-secondary">
                    <CollapsibleTrigger asChild>
                      <button
                        type="button"
                        aria-expanded={advancedOpen}
                        className="flex items-center justify-between w-full px-2 py-4 rounded transition-colors bg-accent/60 hover:bg-accent/80 group focus:outline-none mb-0">
                        <span className="text-sm font-semibold text-foreground">Optional settings</span>
                        <ChevronDown
                          className={`transition-transform ml-2 ${advancedOpen ? 'rotate-180' : ''} group-hover:text-primary`}
                          size={18}
                        />
                      </button>
                    </CollapsibleTrigger>
                    <CollapsibleContent>
                      <div className="flex flex-col space-y-2 py-2">
                        {validationErrors && <AlertMessage type="error">{validationErrors}</AlertMessage>}
                        <Tabs defaultValue="general">
                          <TabsList className="w-full">
                            <TabsTrigger value="general">General</TabsTrigger>
                            <TabsTrigger value="resources">Resources & Policies</TabsTrigger>
                            <TabsTrigger value="commands">Commands</TabsTrigger>
                          </TabsList>
                          <TabsContent value="general" className="flex flex-col space-y-2">
                            <FormField
                              control={form.control}
                              name="name"
                              render={({ field }) => (
                                <FormItem className="flex flex-col sm:flex-row sm:items-baseline">
                                  <div className="flex-1">
                                    <FormControl>
                                      <Input
                                        type="text"
                                        placeholder="Container name"
                                        className="rounded-sm focus-visible:ring-transparent"
                                        {...field}
                                      />
                                    </FormControl>
                                    <p className="text-xs text-muted">
                                      A random name is generated if you do not provide one.
                                    </p>
                                    <FormMessage className="text-xs" />
                                  </div>
                                </FormItem>
                              )}
                            />
                            {imageInfoIsSuccess && imageInfo?.data.exposedPorts && (
                              <PortMappingInput
                                control={control}
                                name="ports"
                                ports={imageInfo?.data.exposedPorts ?? []}
                              />
                            )}

                            {imageInfoIsSuccess && imageInfo?.data.networks && (
                              <FormField
                                control={control}
                                name="networks"
                                render={({ field }) => (
                                  <FormItem>
                                    <FormLabel className="text-xs">Networks</FormLabel>
                                    <MultiSelect
                                      popoverClassName="w-[var(--radix-popper-anchor-width)]"
                                      searchable={false}
                                      modalPopover={true}
                                      options={(imageInfo.data.networks ?? []).map((n: string) => ({
                                        label: n,
                                        value: n,
                                      }))}
                                      onValueChange={field.onChange}
                                      value={field.value ?? []}
                                      placeholder="Select networks to connect this deployment to…"
                                    />
                                    <FormMessage />
                                  </FormItem>
                                )}
                              />
                            )}
                            {imageInfoIsSuccess && imageInfo?.data.volumes && (
                              <VolumeMappingInput
                                name="volumes"
                                fields={volumesFields}
                                control={control}
                                append={appendVolume}
                                remove={removeVolume}
                                containerVolumes={imageInfo?.data.volumes ?? []}
                              />
                            )}

                            <KeyValuePairInput
                              name="envVars"
                              fields={envVarsFields}
                              control={control}
                              append={appendEnvVar}
                              remove={removeEnvVar}
                              label="Environement variables"
                              addButtonLabel="Add environement variable"
                              keyPlaceHolder="key"
                              valuePlaceHolder="value"
                            />
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
                          </TabsContent>
                          <TabsContent value="resources" className="flex flex-col space-y-4">
                            {imageInfoIsSuccess && imageInfo?.data.memTotal && (
                              <FormField
                                control={form.control}
                                name="memoryReservation"
                                render={({ field }) => (
                                  <FormItem>
                                    <FormLabel className="text-xs">Memory Reserveration (MB)</FormLabel>
                                    <div className="flex items-center space-x-2">
                                      <FormControl>
                                        <Slider
                                          value={[field.value || 0]}
                                          max={Math.round((imageInfo.data.memTotal as number) / 1024 / 1024)}
                                          step={1}
                                          className="w-[90%]"
                                          onValueChange={(value) => field.onChange(value[0])}
                                        />
                                      </FormControl>
                                      <span className="text-xs">{field.value} MB</span>
                                    </div>
                                    <FormMessage />
                                  </FormItem>
                                )}
                              />
                            )}
                            {imageInfoIsSuccess && imageInfo?.data.memTotal && (
                              <FormField
                                control={form.control}
                                name="memoryLimit"
                                render={({ field }) => (
                                  <FormItem>
                                    <FormLabel className="text-xs">Memory limit (MB)</FormLabel>
                                    <div className="flex items-center space-x-2">
                                      <FormControl>
                                        <Slider
                                          value={[field.value || 0]}
                                          max={Math.round((imageInfo.data.memTotal as number) / 1024 / 1024)}
                                          step={1}
                                          className="w-[90%]"
                                          onValueChange={(value) => field.onChange(value[0])}
                                        />
                                      </FormControl>
                                      <span className="text-xs">{field.value} MB</span>
                                    </div>
                                    <FormMessage />
                                  </FormItem>
                                )}
                              />
                            )}
                            {imageInfoIsSuccess && imageInfo?.data.cpuCount && (
                              <FormField
                                control={form.control}
                                name="cpuLimit"
                                render={({ field }) => (
                                  <FormItem>
                                    <FormLabel className="text-xs">CPU limit</FormLabel>
                                    <div className="flex items-center space-x-2">
                                      <FormControl>
                                        <Slider
                                          value={[field.value || 0]}
                                          max={imageInfo.data.cpuCount as number}
                                          step={0.1}
                                          className="w-[90%]"
                                          onValueChange={(value) => field.onChange(value[0])}
                                        />
                                      </FormControl>
                                      <span className="text-xs">{field.value} CPUs</span>
                                    </div>
                                    <FormMessage />
                                  </FormItem>
                                )}
                              />
                            )}
                            <FormField
                              control={form.control}
                              name="restartPolicy"
                              render={({ field }) => (
                                <FormItem>
                                  <div className="flex items-center">
                                    <FormLabel className="text-xs">Restart Policy</FormLabel>
                                    <TooltipProvider>
                                      <Tooltip>
                                        <TooltipTrigger>
                                          <Info className="ml-1 h-3 w-3" />
                                        </TooltipTrigger>
                                        <TooltipContent>
                                          <p>
                                            The behavior to apply when the container exits. The default is not to
                                            restart
                                          </p>
                                        </TooltipContent>
                                      </Tooltip>
                                    </TooltipProvider>
                                  </div>
                                  <Select onValueChange={field.onChange} defaultValue={field.value}>
                                    <FormControl>
                                      <SelectTrigger className="w-full">
                                        <SelectValue placeholder="Select a restart policy" />
                                      </SelectTrigger>
                                    </FormControl>
                                    <SelectContent className="bg-background">
                                      {RESTART_POLICIES.map((policy) => (
                                        <SelectItem key={policy.value} value={policy.value}>
                                          {policy.label}
                                        </SelectItem>
                                      ))}
                                    </SelectContent>
                                  </Select>
                                  <FormMessage />
                                </FormItem>
                              )}
                            />
                            <FormField
                              control={form.control}
                              name="autoRemove"
                              render={({ field }) => (
                                <FormItem className="col-span-2 flex flex-row items-center justify-between">
                                  <div>
                                    <FormLabel className="text-xs">Auto-remove on exit</FormLabel>
                                    <TooltipProvider>
                                      <Tooltip>
                                        <TooltipTrigger>
                                          <Info className="ml-1 h-3 w-3" />
                                        </TooltipTrigger>
                                        <TooltipContent>
                                          <p>
                                            Automatically remove the container when the container&apos;s process exits.
                                            This has no effect if RestartPolicy is set.
                                          </p>
                                        </TooltipContent>
                                      </Tooltip>
                                    </TooltipProvider>
                                  </div>
                                  <FormControl>
                                    <Switch checked={field.value} onCheckedChange={field.onChange} />
                                  </FormControl>
                                  <FormMessage className="text-xs" />
                                </FormItem>
                              )}
                            />
                          </TabsContent>
                          <TabsContent value="commands" className="flex flex-col space-y-2">
                            <FormField
                              control={form.control}
                              name="workingDir"
                              render={({ field }) => (
                                <FormItem className="flex flex-col sm:flex-row sm:items-baseline">
                                  <div className="flex-1">
                                    <FormLabel className="text-xs">Working Directory</FormLabel>
                                    <FormControl>
                                      <Input
                                        type="text"
                                        placeholder="e.g. /myapp"
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
                              name="user"
                              render={({ field }) => (
                                <FormItem className="flex flex-col sm:flex-row sm:items-baseline">
                                  <div className="flex-1">
                                    <FormLabel className="text-xs">User</FormLabel>
                                    <FormControl>
                                      <Input
                                        type="text"
                                        placeholder="e.g. myuser"
                                        className="rounded-sm focus-visible:ring-transparent"
                                        {...field}
                                      />
                                    </FormControl>

                                    <FormMessage className="text-xs" />
                                  </div>
                                </FormItem>
                              )}
                            />
                            <ValueInput
                              name="entryPoint"
                              fields={entryPointFields}
                              control={control}
                              append={appendEntryPoint}
                              remove={removeEntryPoint}
                              label="Entry Point"
                              addButtonLabel="Add entry point"
                              valuePlaceHolder="e.g. /bin/sh"
                              helpText="The entry point for the container as a string or an array of strings."
                            />
                            <ValueInput
                              name="command"
                              fields={commandFields}
                              control={control}
                              append={appendCommand}
                              remove={removeCommand}
                              label="Command"
                              addButtonLabel="Add command"
                              valuePlaceHolder="e.g. --housekeeping_interval=5s"
                              helpText="Command to run specified as a string or an array of strings."
                            />
                          </TabsContent>
                        </Tabs>
                      </div>
                    </CollapsibleContent>
                  </div>
                </Collapsible>
              </div>
              <DialogFooter>
                <div className="flex items-center justify-end">
                  <button
                    type="button"
                    onClick={() => onOpenChange(false)}
                    className="text-foreground bg-secondary hover:bg-secondary/80 font-medium rounded-sm text-sm px-2 py-2">
                    Cancel
                  </button>
                  <button
                    type="submit"
                    disabled={isPending || imageInfoIsLoading}
                    className="ml-2 bg-primary hover:bg-primary/85 text-background font-medium rounded-sm text-sm inline-flex items-center px-2 py-2">
                    Create
                    {isPending && <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />}
                  </button>
                </div>
              </DialogFooter>
            </form>
          </Form>
        )}
      </DialogContent>
    </Dialog>
  );
};
