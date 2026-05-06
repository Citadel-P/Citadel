import {
  CreateDeploymentInput,
  PlatformView,
  ImageView,
  DeploymentImageInfoExternalImage,
  DeploymentImageInfoLocalImage,
  DockerNetworkResult,
  ContainerRestartPolicy,
  ResourceSpec,
  StopSignal,
  UpdateBehavior,
  DeploymentConfigView,
  PatchDeploymentInput,
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
import { useState, useMemo, useEffect, useCallback, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { useParams } from 'react-router';
import { MultiResourceSelectorField, ResourceSelectorField } from '@/components/custom/common';
import { MonacoToArrayEditor, MonacoToDictionaryEditor } from '@/lib/monaco';
import { AlertMessage } from '@/components/custom/alert-message';

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

const update_behaviors = {
  [UpdateBehavior.Disabled]: {
    label: 'Disabled',
    description: 'Do not check for updates.',
  },
  [UpdateBehavior.Notify]: {
    label: 'Notify Only',
    description: 'Periodically check for updates and alert me, but do not redeploy.',
  },
  [UpdateBehavior.AutoDeploy]: {
    label: 'Auto Deploy',
    description: 'Periodically check and automatically redeploy when a new image is found.',
  },
};

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
    spec: { nanoCpus: 0.25, memoryLimit: 256 },
  },
  [ResourceProfile.small]: {
    label: 'Small',
    description: '0.5 CPU - 512 MB RAM',
    spec: { nanoCpus: 0.5, memoryLimit: 512 },
  },
  [ResourceProfile.medium]: {
    label: 'Medium',
    description: '0.5 CPU - 1 GB RAM',
    spec: { nanoCpus: 0.5, memoryLimit: 1024 },
  },
  [ResourceProfile.large]: {
    label: 'Large',
    description: '1.0 CPU - 2 GB RAM',
    spec: { nanoCpus: 1, memoryLimit: 2048 },
  },
  [ResourceProfile.xlarge]: {
    label: 'X-Large',
    description: '2.0 CPU - 4 GB RAM',
    spec: { nanoCpus: 2, memoryLimit: 4096 },
  },
};

const stop_signals = {
  [StopSignal.SIGTERM]: {
    label: StopSignal.SIGTERM,
    description: 'Request a graceful shutdown. Default and recommended.',
  },
  [StopSignal.SIGINT]: {
    label: StopSignal.SIGINT,
    description: 'Gracefully interrupt the process (similar to Ctrl+C).',
  },
  [StopSignal.SIGKILL]: {
    label: StopSignal.SIGKILL,
    description: 'Forcefully stop the process immediately.',
  },
};

type DeploymentInput = CreateDeploymentInput | PatchDeploymentInput;

export const DeploymentForm = ({ mode, metadataChanged }: { mode: 'add' | 'edit'; metadataChanged?: boolean }) => {
  const id = useParams().id;
  const [update, setUpdate] = useState<Partial<DeploymentInput>>({});
  const queryClient = useQueryClient();

  const { mutateAsync: createDeployment } = useMutate('createDeployment');
  const { mutateAsync: updateDeployment } = useMutate('updateDeployment');
  const { data: deploymentCfg } = useRead('getDeploymentConfig', { deploymentId: id });

  const resource: DeploymentConfigView | undefined = deploymentCfg?.data;

  const original = resource ?? ({} as DeploymentConfigView);

  // Fallback Logic: Check `update` first, then `original`.
  const currentPlatformId = update.platformId ?? original.platformId;
  const currentSpec = { ...original.spec, ...update.spec };
  const currentImage = update.spec?.image ?? original.spec?.image;
  const provider = currentImage?.$type;

  const { data, isSuccess: imageInfoIsSuccess } = useRead('getExposedPorts', {
    platformId: currentPlatformId,
    imageId: (currentImage as DeploymentImageInfoLocalImage)?.imageId,
  });
  const lastAppliedServerPortsRef = useRef<string[] | null>(null);

  useEffect(() => {
    const shouldAutoFill =
      imageInfoIsSuccess && data?.data?.ports !== undefined && currentImage?.$type === ImageSource.local;

    if (!shouldAutoFill) return;

    const serverPorts = data.data.ports ?? [];
    const userPorts = update.spec?.ports;
    const lastApplied = lastAppliedServerPortsRef.current;
    if (userPorts !== undefined && lastApplied && JSON.stringify(userPorts) !== JSON.stringify(lastApplied)) {
      return;
    }

    // No update needed if server ports match what we already applied
    if (lastApplied && JSON.stringify(serverPorts) === JSON.stringify(lastApplied)) {
      return;
    }

    lastAppliedServerPortsRef.current = serverPorts;
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
  }, [imageInfoIsSuccess, data?.data?.ports, currentImage?.$type, update.spec?.ports, original.spec?.ports]);

  const refreshData = useCallback(() => {
    localStorage.removeItem(`deployment:${id ?? 'new'}`);
    queryClient.invalidateQueries({ queryKey: ['getDeploymentConfig', { deploymentId: id }] });
  }, [id, queryClient]);

  useEffect(() => {
    if (!metadataChanged) return;
    refreshData();
  }, [metadataChanged, refreshData]);

  const { save: handleSave, isPending } = useSaveResource<DeploymentInput, any>({
    mode,
    basePath: 'deployments',
    entityName: 'Deployment',
    onCreate: (payload) => createDeployment({ data: payload as CreateDeploymentInput }),
    onUpdate: (payload) => updateDeployment({ id, data: payload }),
    onRefresh: refreshData,
  });

  const schema = useMemo(
    () => ({
      general: defineSection<DeploymentInput>({
        title: '',
        items: [
          ...(mode === 'add'
            ? [
                defineGroupField<DeploymentInput>({
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
                        <FieldInput
                          value={val}
                          onChange={(v) => set({ name: v })}
                          placeholder="e.g. production-web-server"
                        />
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
              ]
            : []),
          defineField({
            key: 'platformId',
            label: 'Platform',
            required: true,
            disabled: false,
            description: 'Select the platform to deploy on.',
            render: (value, set) => {
              return (
                <ResourceSelectorField
                  type="Platform"
                  selected={value}
                  onSelect={(v: PlatformView | undefined) => set({ platformId: v?.id })}
                  placeholder="Select Platform"
                />
              );
            },
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
                render: (val, set) => {
                  return (
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
                  );
                },
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
                              platformId={currentPlatformId}
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
                              className="sm:min-w-100"
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
                        platformId={currentPlatformId}
                        onSelect={(v: ImageView | undefined) => {
                          lastAppliedServerPortsRef.current = null;
                          set((prev) => ({
                            spec: {
                              ...prev.spec!,
                              image: {
                                $type: 'Local',
                                ...((prev.spec?.image as DeploymentImageInfoLocalImage) ?? {}),
                                imageId: v?.dockerImageId ?? '',
                              } satisfies DeploymentImageInfoLocalImage,
                              ports: [],
                            },
                          }));
                        }}
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
                          networks: v?.map((s) => s.name) ?? [],
                        },
                      }))
                    }
                    placeholder="Select Network(s)"
                    platformId={currentPlatformId}
                    valueKey="name"
                  />
                ),
              }),
              provider === ImageSource.local
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
                      <MonacoToArrayEditor
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
              <MonacoToArrayEditor
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
          defineField({
            key: 'spec.envVars',
            label: 'Environment',
            description: 'Runtime configuration passed to the application inside the container.',
            required: false,
            render: (value, set) => (
              <MonacoToArrayEditor
                value={value}
                helperText="# KEY=value"
                language="key_value"
                onChange={(e: string[] | undefined) =>
                  set((prev) => ({
                    spec: {
                      ...prev.spec!,
                      envVars: e ?? [],
                    },
                  }))
                }
              />
            ),
          }),
          defineField({
            key: 'spec.updateBehavior',
            label: 'Auto Update',
            description: 'Define how the platform handles new image versions.',
            render: (value, set) => {
              let disabled = provider === ImageSource.local;
              let warningMsg = 'Auto update requires an external image source.';

              if (
                provider === ImageSource.external &&
                (currentSpec.image as DeploymentImageInfoExternalImage)?.imageTag?.includes('@')
              ) {
                disabled = true;
                warningMsg = "Cannot enable Auto-update for an image pinned by digest (contains '@')";
              }
              return (
                <div className="flex flex-col gap-2">
                  <ItemSelector
                    collection={update_behaviors}
                    value={value}
                    disabled={disabled}
                    onChange={(updateBehavior: UpdateBehavior) => {
                      set((prev) => ({
                        spec: {
                          ...prev.spec!,
                          updateBehavior: updateBehavior,
                        },
                      }));
                    }}
                  />
                  {disabled && (
                    <AlertMessage type={'warning'} title={''}>
                      <span className=" font-normal">{warningMsg}</span>
                    </AlertMessage>
                  )}
                </div>
              );
            },
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
                    ([_, p]) => p.spec?.nanoCpus === spec?.nanoCpus && p.spec?.memoryLimit === spec?.memoryLimit,
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
          defineGroupField({
            id: 'lifecycle',
            label: 'Lifecycle',
            title: 'Lifecycle',
            description: 'Manage container reliability and shutdown behavior.',
            items: [
              defineField({
                key: 'spec.lifeCycleSpec.restartPolicy',
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
                          lifeCycleSpec: {
                            ...(prev.spec?.lifeCycleSpec as any),
                            restartPolicy: policy,
                          },
                        },
                      }))
                    }
                  />
                ),
              }),
              defineField({
                key: 'spec.lifeCycleSpec.stopSignal',
                label: 'Stop Signal',
                description: 'Signal sent to the container to initiate shutdown.',
                render: (value, set) => (
                  <ItemSelector
                    collection={stop_signals}
                    value={value}
                    onChange={(sig: ContainerRestartPolicy) =>
                      set((prev) => ({
                        spec: {
                          ...(prev.spec as DeploymentInput['spec']),
                          lifeCycleSpec: {
                            ...(prev.spec?.lifeCycleSpec as any),
                            stopSignal: sig,
                          },
                        },
                      }))
                    }
                  />
                ),
              }),

              defineField({
                key: 'spec.lifeCycleSpec.stopTimeout',
                label: 'Stop Timeout',
                description:
                  'Maximum time to wait (in seconds) for the container to stop before it is forcefully terminated.',
                render: (val, set) => (
                  <FieldInput
                    type="number"
                    value={val}
                    onChange={(v) =>
                      set((prev) => ({
                        spec: {
                          ...(prev.spec as DeploymentInput['spec']),
                          lifeCycleSpec: {
                            ...(prev.spec?.lifeCycleSpec as any),
                            stopTimeout: v,
                          },
                        },
                      }))
                    }
                    placeholder="e.g. 10"
                  />
                ),
              }),
            ],
          }),
          defineField({
            label: 'Command',
            key: 'spec.command',
            description: 'Overrides the image default command.',
            required: false,
            render: (value, set) => (
              <MonacoToArrayEditor
                value={value}
                helperText="# --housekeeping_interval=5s"
                language="key_value"
                onChange={(cmd: string[] | undefined) =>
                  set((prev) => ({
                    spec: {
                      ...prev.spec!,
                      command: cmd ?? [],
                    },
                  }))
                }
              />
            ),
          }),

          defineField({
            key: 'spec.labels',
            label: 'Labels',
            description: 'User-defined key/value metadata.',
            render: (value, set) => (
              <MonacoToDictionaryEditor
                value={value}
                helperText="# KEY=value"
                language="key_value"
                onChange={(e: Record<string, string> | undefined) =>
                  set((prev) => ({
                    spec: {
                      ...prev.spec!,
                      labels: e ?? {},
                    },
                  }))
                }
              />
            ),
          }),
        ],
      }),
    }),
    [provider, currentPlatformId, currentSpec.image, mode],
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
