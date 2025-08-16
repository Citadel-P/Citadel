import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog';

import { useAppContext } from '@/AppContext';
import { useImagesContext } from '../ImagesContext';
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from '@/components/ui/collapsible';
import KeyValuePairInput from '@/components/ui/KeyValuePairInput';
import { LoaderCircle, ChevronDown } from 'lucide-react';
import { useFieldArray } from 'react-hook-form';
import { DialogFooter } from '@/components/ui/dialog';
import { useEffect, useState } from 'react';
import { useRunImageForm } from '../hooks/useRunImageForm';
import { Form, FormControl, FormField, FormItem, FormMessage } from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { useGETImageInfo } from '../hooks/useGETImageInfo';

import PortMappingInput from '../components/PortMappingInput';
import VolumeMappingInput from '../components/VolumeMappingInput';

export const RunImageDialog = () => {
  const { currentPlatform } = useAppContext();
  const { runDialogData, setRunDialogData } = useImagesContext();
  const {
    data: imageInfo,
    isLoading: imageInfoIsLoading,
    isSuccess: imageInfoIsSuccess,
  } = useGETImageInfo(currentPlatform?.id, runDialogData.currentSelection?.at(0)?.id);
  const { form } = useRunImageForm();
  const { control } = form;
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const runIsPending = false;

  useEffect(() => {
    if (imageInfoIsSuccess) {
      const exposedPorts = imageInfo?.data.exposedPorts;
      if (exposedPorts) {
        form.setValue('ports', exposedPorts);
      }
    }
  }, [imageInfo, imageInfoIsSuccess, form]);
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

  function onSubmit(values: any) {
    const finalValues = {
      ...values,
      volumes: values.volumes
        .filter((v: any) => v.hostPath && v.containerPath)
        .map((v: any) => `${v.hostPath}:${v.containerPath}`),
    };
    console.log(finalValues);
  }

  function onOpenChange(open: boolean) {
    if (!open) {
      form.reset();
      setAdvancedOpen(false);
      labelFields.map((_, i) => removeLabel(i));
      envVarsFields.map((_, i) => removeEnvVar(i));
      volumesFields.map((_, i) => removeVolume(i));
    }
    setRunDialogData({ open });
  }

  return (
    <Dialog open={runDialogData.open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[700px]" onOpenAutoFocus={(e) => e.preventDefault()}>
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)}>
            <DialogHeader>
              <DialogTitle>Run a new container</DialogTitle>
              <DialogDescription>
                {runDialogData.currentSelection?.at(0)?.name ?? '-'}:{runDialogData.currentSelection?.at(0)?.tag ?? '-'}
              </DialogDescription>
            </DialogHeader>
            <div className="grid gap-4 py-4">
              <Collapsible open={advancedOpen} onOpenChange={setAdvancedOpen}>
                <div className="flex flex-col space-y-2 border-t-1 border-b-1 border-secondary">
                  <CollapsibleTrigger asChild>
                    <button
                      type="button"
                      aria-expanded={advancedOpen}
                      className="flex items-center justify-between w-full px-2 py-4 rounded transition-colors hover:bg-accent group focus:outline-none mb-0">
                      <span className="text-sm font-semibold text-foreground group-hover:text-primary transition-colors">
                        Optional settings
                      </span>
                      <ChevronDown
                        className={`transition-transform ml-2 ${advancedOpen ? 'rotate-180' : ''} group-hover:text-primary`}
                        size={18}
                      />
                    </button>
                  </CollapsibleTrigger>
                  <CollapsibleContent>
                    <div className="flex flex-col space-y-2 py-2">
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
                        <PortMappingInput control={control} name="ports" ports={imageInfo?.data.exposedPorts ?? []} />
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
                        keyPlaceHolder="Variable"
                        valuePlaceHolder="Value"
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
                  disabled={runIsPending || imageInfoIsLoading}
                  className="ml-2 bg-primary hover:bg-primary/85 text-background font-medium rounded-sm text-sm inline-flex items-center px-2 py-2">
                  Run
                  {runIsPending && <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />}
                </button>
              </div>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
};
