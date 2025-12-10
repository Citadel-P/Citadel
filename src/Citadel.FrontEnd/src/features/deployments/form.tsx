import {
  DeploymentInput,
  PlatformView,
  ImageView,
  DeploymentImageInfoExternalImage,
  DeploymentImageInfoLocalImage,
  DockerNetworkResult,
} from '@/api/generated/api.types';
import {
  FormShell,
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldTextArea,
  defineRowField,
  PortMappingField,
} from '@/components/custom/form-builder';
import { Constants } from '@/lib/constants';
import { useState, useMemo, useEffect } from 'react';
import { useMutate, useRead } from '@/lib/hooks';
import { toast } from 'sonner';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useParams, useNavigate } from 'react-router';
import { MultiResourceSelectorField, ResourceSelectorField } from '@/components/custom/common';

const enum ImageSource {
  Local = 'Local',
  External = 'External',
}

const imageSource = {
  Local: {
    label: ImageSource.Local,
    description: 'Use an image that already exists on the target Docker host.',
  },
  External: {
    label: ImageSource.External,
    description: 'Pull an image from a remote registry (Docker Hub, GitHub, etc.)',
  },
};

const ImageSourceSelector = ({ value, onChange, disabled }: any) => {
  const finalValue = value ?? ImageSource.Local;
  const selected = imageSource[finalValue as keyof typeof imageSource];
  return (
    <Select value={finalValue} onValueChange={onChange} disabled={disabled}>
      <SelectTrigger className="w-full max-w-[400px]">
        <SelectValue>
          {selected ? (
            <div className="flex items-center gap-2">
              <span>{selected.label}</span>
            </div>
          ) : (
            'Select image source'
          )}
        </SelectValue>
      </SelectTrigger>

      <SelectContent className="bg-background">
        {Object.entries(imageSource).map(([key, info]) => (
          <SelectItem key={key} value={key}>
            <div className="flex items-center gap-2">
              <div className="flex flex-col">
                <span className="font-medium">{info.label}</span>
                <span className="text-xs text-muted-foreground">{info.description}</span>
              </div>
            </div>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
};

export const DeploymentForm = ({ mode, resource }: { mode: 'add' | 'edit'; resource?: DeploymentInput }) => {
  const id = useParams().id;
  const navigate = useNavigate();
  const [update, setUpdate] = useState<Partial<DeploymentInput>>({});
  const [isPending, setIsPending] = useState(false);

  const { data, isSuccess: imageInfoIsSuccess } = useRead('getExposedPorts', {
    platformId: update.platformId,
    imageId: (update.spec?.image as DeploymentImageInfoLocalImage)?.imageId,
  });

  useEffect(() => {
    if (imageInfoIsSuccess && data?.data?.ports !== undefined) {
      setUpdate(
        (prev) =>
          ({
            ...prev,
            spec: {
              ...(prev.spec ?? {}),
              ports: data.data.ports,
            },
          }) as Partial<DeploymentInput>,
      );
    }
  }, [imageInfoIsSuccess, data?.data?.ports]);

  const { mutateAsync: createDeployment } = useMutate('createDeployment');
  const { mutateAsync: updateDeployment } = useMutate('updateDeployment');

  const original = resource ?? ({} as DeploymentInput);
  const provider = update.spec?.image?.$type;

  const handleSave = async (payload: DeploymentInput) => {
    setIsPending(true);
    try {
      if (mode === 'edit') await updateDeployment({ id, data: payload });
      else await createDeployment({ data: payload });

      toast.success(`Deployment "${payload.name}" saved successfully`);
      navigate('/deployments');
    } finally {
      setIsPending(false);
    }
  };

  const schema = useMemo(
    () => ({
      general: defineSection<DeploymentInput>({
        title: '',
        items: [
          defineGroupField({
            id: 'details',
            label: 'Details',
            items: [
              defineField({
                key: 'name',
                label: 'Name',
                required: true,
                description: 'Internal identifier for this workload.',
                validate: (v) => (!v ? 'Name is required' : null),
                render: (val, set) => (
                  <FieldInput value={val} onChange={(v) => set({ name: v })} placeholder="e.g. production-web-server" />
                ),
              }),

              defineField({
                key: 'description',
                label: 'Description',
                required: false,
                description: 'Optional description of this workload.',
                render: (val, set) => <FieldTextArea value={val} onChange={(v) => set({ description: v })} />,
              }),
            ],
          }),
          defineField({
            key: 'platformId',
            label: 'Platform',
            required: true,
            disabled: false,
            description: 'Select the platform to deploy on.',
            render: (value, set) => (
              <ResourceSelectorField
                type="Platform"
                selected={value}
                onSelect={(v: PlatformView | undefined) => set({ platformId: v?.id })}
                placeholder="Select Platform"
              />
            ),
          }),
          defineGroupField<DeploymentInput>({
            id: 'image',
            label: 'Image',
            items: [
              defineField({
                label: 'Image Source',
                key: 'spec.image.$type',
                description: 'Select the image source.',
                required: true,
                validate: (v) => (!v ? 'Source is required' : null),
                render: (val, set) => (
                  <ImageSourceSelector
                    value={val}
                    onChange={(v: ImageSource) => set({ spec: { image: { $type: v } } as any })}
                  />
                ),
              }),

              provider === ImageSource.External
                ? defineRowField({
                    id: 'imageRow',
                    gap: 'gap-8',
                    fields: [
                      defineField({
                        key: 'spec.image.registryId',
                        label: 'Registry',
                        required: true,
                        description: 'Select the registry to pull the image from.',
                        render: (val, set) => {
                          return (
                            <ResourceSelectorField
                              type="Registry"
                              selected={val}
                              platformId={update.platformId}
                              onSelect={(v: ImageView | undefined) =>
                                set((prev) => ({
                                  spec: {
                                    ...prev.spec!,
                                    image: {
                                      $type: 'External',
                                      ...((prev.spec?.image as DeploymentImageInfoExternalImage) ?? {}),
                                      registryId: v?.id ?? '',
                                    } satisfies DeploymentImageInfoExternalImage,
                                  },
                                }))
                              }
                              placeholder="Select Registry"
                              className="sm:min-w-[400px]"
                            />
                          );
                        },
                      }),
                      defineField({
                        key: 'spec.image.imageTag',
                        label: ' ',
                        required: true,
                        description: 'Enter the image reference.',
                        render: (val, set) => {
                          return (
                            <FieldInput
                              value={val}
                              onChange={(v) =>
                                set((prev) => ({
                                  spec: {
                                    ...prev.spec!,
                                    image: {
                                      $type: 'External',
                                      ...((prev.spec?.image as DeploymentImageInfoExternalImage) ?? {}),
                                      imageTag: v,
                                    } satisfies DeploymentImageInfoExternalImage,
                                  },
                                }))
                              }
                              placeholder="e.g. nginx:latest "
                              className="w-full max-w-full"
                            />
                          );
                        },
                      }),
                    ],
                  })
                : defineField({
                    key: 'spec.image.imageId',
                    label: `Local Image`,
                    required: true,
                    description: 'These images are immediately available for deployment without a remote pull.',
                    render: (val, set) => (
                      <ResourceSelectorField
                        type="Image"
                        selected={val}
                        platformId={update.platformId}
                        onSelect={(v: ImageView | undefined) =>
                          set((prev) => ({
                            spec: {
                              ...prev.spec!,
                              image: {
                                $type: 'Local',
                                ...((prev.spec?.image as DeploymentImageInfoLocalImage) ?? {}),
                                imageId: v?.id ?? '',
                              } satisfies DeploymentImageInfoLocalImage,
                            },
                          }))
                        }
                        placeholder="Select Image"
                      />
                    ),
                  }),
            ],
          }),
          defineGroupField<DeploymentInput>({
            id: 'networks',
            label: 'Networks',
            items: [
              defineField({
                label: 'Networks',
                key: 'spec.networks',
                description: 'Select the Docker networks that this container will attach to.',
                required: true,
                validate: (v) => (!v ? 'Source is required' : null),
                render: (value, set) => (
                  <MultiResourceSelectorField
                    type="Network"
                    selected={value}
                    onSelect={(v: DockerNetworkResult[] | undefined) =>
                      set((prev) => ({
                        spec: {
                          ...prev.spec!,
                          networks: v?.map((s) => s.id) ?? [],
                        },
                      }))
                    }
                    placeholder="Select Network(s)"
                    platformId={update.platformId}
                  />
                ),
              }),
              defineField({
                label: 'Ports',
                key: 'spec.ports',
                description: 'Configure port mappings.',
                required: false,
                render: (value, set) => (
                  <PortMappingField
                    ports={value ?? []}
                    set={(v: string[] | undefined) =>
                      set((prev) => ({
                        spec: {
                          ...prev.spec!,
                          ports: v?.map((s) => s) ?? [],
                        },
                      }))
                    }
                  />
                ),
              }),
            ],
          }),
        ],
      }),
      '': defineSection<DeploymentInput>({
        title: '',
        items: [
          defineField({
            key: 'name',
            label: 'Name',
            description: 'Provide a unique name to identify this registry.',
            required: true,
            validate: (v) => (!new RegExp(Constants.validNameIdentifier).test(v) ? 'Invalid name format' : null),
            render: (value, set) => (
              <FieldInput value={value ?? ''} onChange={(v) => set({ name: v })} placeholder="my-registry" />
            ),
          }),
        ],
      }),
    }),
    [provider, update.platformId],
  );

  return (
    <FormShell
      mode={mode}
      schema={schema}
      original={original}
      update={update}
      setUpdate={setUpdate}
      onSave={handleSave}
      pending={isPending}
      draftKey={`deployment:${id ?? 'new'}`}
      draftVersion={1}
    />
  );
};
