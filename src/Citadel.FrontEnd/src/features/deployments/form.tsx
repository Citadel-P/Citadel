import {
  DeploymentInput,
  PlatformView,
  ImageView,
  DeploymentImageInfoExternalImage,
  DeploymentImageInfoLocalImage,
  DockerNetworkResult,
  ContainerRestartPolicy,
  ResourceSpec,
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
  ItemSelector,
} from '@/components/custom/form-builder';
import { useState, useMemo, useEffect } from 'react';
import { useMutate, useRead } from '@/lib/hooks';
import { toast } from 'sonner';
import { useParams, useNavigate } from 'react-router';
import { MultiResourceSelectorField, ResourceSelectorField } from '@/components/custom/common';
import { MonacoToArrayStringEditor } from '@/lib/monaco';

const enum ImageSource {
  local = 'Local',
  external = 'External',
}

const enum ResourceProfile {
  automatic,
  xsmall,
  small,
  medium,
  large,
  xlarge,
}

const image_source = {
  [ImageSource.local]: {
    label: 'Local',
    description: 'Use an image that already exists on the target Docker host.',
  },
  [ImageSource.external]: {
    label: 'External',
    description: 'Pull an image from a remote registry (Docker Hub, GitHub, etc.)',
  },
};

const restart_policies = {
  No: { label: 'No', description: 'Do not automatically restart' },
  Always: {
    label: 'Always',
    description: 'Automatically restarts the container whenever it stops.',
  },
  UnlessStopped: {
    label: 'Unless Stopped',
    description: 'Restart always except when the user has manually stopped the container',
  },
  OnFailure: {
    label: 'On Failure',
    description: 'Restart only when the container exit code is non-zero',
  },
};

const resource_profiles = {
  [ResourceProfile.automatic]: {
    label: 'Automatic',
    description: 'Resources will be allocated automatically by the platform',
    spec: null,
  },
  [ResourceProfile.xsmall]: {
    label: 'X-Small',
    description: '0.25 CPU - 256 MB RAM',
    spec: { cpuLimit: 0.25, memoryLimit: 256, memoryReservation: null },
  },
  [ResourceProfile.small]: {
    label: 'Small',
    description: '0.5 CPU - 512 MB RAM',
    spec: { cpuLimit: 0.5, memoryLimit: 512, memoryReservation: null },
  },
  [ResourceProfile.medium]: {
    label: 'Medium',
    description: '0.5 CPU - 1 GB RAM',
    spec: { cpuLimit: 0.5, memoryLimit: 1024, memoryReservation: null },
  },
  [ResourceProfile.large]: {
    label: 'Large',
    description: '1.0 CPU - 2 GB RAM',
    spec: { cpuLimit: 1, memoryLimit: 2048, memoryReservation: null },
  },
  [ResourceProfile.xlarge]: {
    label: 'X-Large',
    description: '2.0 CPU - 4 GB RAM',
    spec: { cpuLimit: 2, memoryLimit: 4096, memoryReservation: null },
  },
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

  const { mutateAsync: createDeployment } = useMutate('createDeployment');
  const { mutateAsync: updateDeployment } = useMutate('updateDeployment');

  const original = resource ?? ({} as DeploymentInput);
  const provider = update.spec?.image?.$type;

  useEffect(() => {
    const shouldFetch =
      imageInfoIsSuccess && data?.data?.ports !== undefined && update.spec?.image?.$type === ImageSource.local;

    if (!shouldFetch) return;

    const currentPorts = update.spec?.ports ?? [];
    const serverPorts = data.data.ports ?? [];

    if (currentPorts.length > 0 && JSON.stringify(currentPorts) !== JSON.stringify(serverPorts)) {
      return;
    }

    if (JSON.stringify(currentPorts) === JSON.stringify(serverPorts)) {
      return;
    }

    setUpdate(
      (prev) =>
        ({
          ...prev,
          spec: {
            ...(prev.spec ?? {}),
            ports: serverPorts,
          },
        }) as Partial<DeploymentInput>,
    );
  }, [imageInfoIsSuccess, data?.data?.ports, update.spec?.image?.$type, update.spec?.ports]);

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
                  <ItemSelector
                    collection={image_source}
                    value={val}
                    onChange={(v: ImageSource) =>
                      set((prev) => ({
                        spec: {
                          ...(prev.spec as DeploymentInput['spec']),
                          image: { $type: v } as any,
                          ports: [],
                        },
                      }))
                    }
                  />
                ),
              }),

              provider === ImageSource.external
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
                              ports: [],
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
                    selected={value ?? []}
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
              update.spec?.image?.$type === ImageSource.local
                ? defineField({
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
                  })
                : defineField({
                    label: 'Ports',
                    key: 'spec.ports',
                    description: 'Configure port mappings.',
                    required: false,
                    render: (value, set) => (
                      <MonacoToArrayStringEditor
                        value={value}
                        helperText="# 8080:8080/tcp"
                        language="key_value"
                        onChange={(p: string[] | undefined) =>
                          set((prev) => ({
                            spec: {
                              ...prev.spec!,
                              ports: p ?? [],
                            },
                          }))
                        }
                      />
                    ),
                  }),
            ],
          }),
          defineField({
            key: 'spec.volumes',
            label: 'Volumes',
            description: 'Configure bind mounts or named volumes.',
            required: false,
            render: (value, set) => (
              <MonacoToArrayStringEditor
                value={value}
                helperText="# my-volume:/data or /config:/etc/config:ro"
                language="string_list"
                onChange={(v: string[] | undefined) =>
                  set((prev) => ({
                    spec: {
                      ...prev.spec!,
                      volumes: v ?? [],
                    },
                  }))
                }
              />
            ),
          }),
        ],
      }),
      Advanced: defineSection<DeploymentInput>({
        title: 'Advanced',
        items: [
          defineField({
            key: 'spec.resourceSpec',
            label: 'Resources',
            description: 'Choose how much CPU and memory to allocate to this deployment.',
            render: (value, set) => {
              const toProfile = (spec: ResourceSpec | undefined) => {
                return (
                  Object.entries(resource_profiles).find(
                    ([_, p]) => p.spec?.cpuLimit === spec?.cpuLimit && p.spec?.memoryLimit === spec?.memoryLimit,
                  )?.[0] ?? 'automatic'
                );
              };
              return (
                <ItemSelector
                  collection={resource_profiles}
                  value={toProfile(value)}
                  onChange={(profile: ResourceProfile) =>
                    set((prev) => ({
                      spec: {
                        ...(prev.spec as DeploymentInput['spec']),
                        resourceSpec: resource_profiles[profile]?.spec ?? null,
                      },
                    }))
                  }
                />
              );
            },
          }),
          defineField({
            key: 'spec.restartPolicy',
            label: 'Restart Policy',
            description: 'The behavior to apply when the container exits.',
            render: (value, set) => (
              <ItemSelector
                collection={restart_policies}
                value={value}
                onChange={(policy: ContainerRestartPolicy) =>
                  set((prev) => ({
                    spec: {
                      ...(prev.spec as DeploymentInput['spec']),
                      restartPolicy: policy,
                    },
                  }))
                }
              />
            ),
          }),
        ],
      }),
    }),
    [provider, update.platformId, update.spec?.image?.$type],
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
