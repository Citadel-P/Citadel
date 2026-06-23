import {
  PlatformView,
  UpdateBehavior,
  StackConfigView,
  CreateStackInput,
  PatchStackInput,
  LookupResourceType,
  StackUpdateBehavior,
  StackSource,
} from '@/api/generated/api.types';
import {
  FormShell,
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldTextArea,
  ItemSelector,
  FieldSwitch,
} from '@/components/custom/form-builder';
import { useState, useMemo, useEffect, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { useParams } from 'react-router';
import { ResourceSelectorField } from '@/components/custom/common';
import { MonacoEditor, MonacoToArrayEditor } from '@/lib/monaco';

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

const stack_source = {
  [StackSource.WebEditor]: {
    label: 'Web Editor',
    description: 'Define and manage the stack configuration directly in the web editor.',
  },
  [StackSource.Git]: {
    label: 'Git',
    description: 'Sync the stack configuration from a Git repository and track changes through version control.',
  },
};

type StackInput = CreateStackInput | PatchStackInput;

export const StackForm = ({
  mode,
  metadataChanged,
  disabled,
}: {
  mode: 'add' | 'edit';
  metadataChanged?: boolean;
  disabled?: boolean;
}) => {
  const id = useParams().id;
  const [update, setUpdate] = useState<Partial<StackInput>>({});
  const queryClient = useQueryClient();

  const { mutateAsync: createStack } = useMutate('createStack');
  const { mutateAsync: updateStack } = useMutate('updateStack');
  const { data: stackCfg } = useRead('getStackConfig', { stackId: id });

  const resource: StackConfigView | undefined = stackCfg?.data;
  const original = resource ?? ({} as StackConfigView);
  const currentStackSource = (update as Partial<CreateStackInput>).stackSource ?? original.stackSource;

  const refreshData = useCallback(() => {
    localStorage.removeItem(`stack:${id ?? 'new'}`);
    queryClient.invalidateQueries({ queryKey: ['getStackConfig', { stackId: id }] });
  }, [id, queryClient]);

  useEffect(() => {
    if (!metadataChanged) return;
    refreshData();
  }, [metadataChanged, refreshData]);

  const { save: handleSave, isPending } = useSaveResource<StackInput, any>({
    mode,
    basePath: 'stacks',
    entityName: 'Stack',
    onCreate: (payload) => createStack({ data: payload as CreateStackInput }),
    onUpdate: (payload) =>
      updateStack({
        id,
        data: {
          platformId: payload.platformId,
          spec: payload.spec,
        } as PatchStackInput,
      }),
    onRefresh: refreshData,
  });

  const schema = useMemo(
    () => ({
      general: defineSection<StackInput>({
        title: '',
        items: [
          ...(mode === 'add'
            ? [
                defineGroupField<StackInput>({
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
                        <FieldInput value={val} onChange={(v) => set({ name: v })} placeholder="stack-name" />
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
                  sourceType={LookupResourceType.Stack}
                  targetType={LookupResourceType.Platform}
                  sourceResourceId={id}
                  selected={value}
                  onSelect={(v: PlatformView | undefined) => set({ platformId: v?.id })}
                  placeholder="Select Platform"
                />
              );
            },
          }),
          defineField({
            label: 'Stack Source',
            key: 'stackSource',
            description: 'Select where the stack configuration is managed.',
            required: true,
            disabled: !!currentStackSource,
            validate: (v) => (!v ? 'Source is required' : null),
            render: (val, set) => {
              return (
                <ItemSelector
                  collection={stack_source}
                  value={val}
                  disabled={disabled || !!currentStackSource}
                  onChange={(v: StackSource) => set({ stackSource: v })}
                />
              );
            },
          }),

          ...(currentStackSource === StackSource.Git
            ? [
                defineGroupField<StackInput>({
                  id: 'git_stack_source',
                  label: 'Git Stack',
                  items: [
                    defineField({
                      key: 'spec.gitRepoId',
                      label: 'Repository',
                      required: true,
                      description: 'Select the Git repository that contains the compose project.',
                      render: (value, set) => (
                        <ResourceSelectorField
                          sourceType={LookupResourceType.Stack}
                          targetType={LookupResourceType.GitRepository}
                          sourceResourceId={id}
                          selected={value}
                          onSelect={(v: { id: string } | undefined) =>
                            set((prev) => ({
                              spec: {
                                ...(prev.spec as any),
                                $type: 'Git',
                                gitRepoId: v?.id ?? '',
                              } as any,
                            }))
                          }
                          placeholder="Select Repository"
                        />
                      ),
                    }),
                    defineField({
                      key: 'spec.branch',
                      label: 'Branch',
                      required: true,
                      description: 'Branch to use for the stack source.',
                      validate: (v) => (!v ? 'Branch is required' : null),
                      render: (value, set) => (
                        <FieldInput
                          value={value}
                          onChange={(v) =>
                            set((prev) => ({
                              spec: {
                                ...(prev.spec as any),
                                $type: 'Git',
                                branch: v,
                              } as any,
                            }))
                          }
                          placeholder="e.g. main"
                        />
                      ),
                    }),
                    defineField({
                      key: 'spec.commitSha',
                      label: 'Commit SHA',
                      description: 'Optional specific commit to pin this stack release.',
                      render: (value, set) => (
                        <FieldInput
                          value={value}
                          onChange={(v) =>
                            set((prev) => ({
                              spec: {
                                ...(prev.spec as any),
                                $type: 'Git',
                                commitSha: v || null,
                              } as any,
                            }))
                          }
                          placeholder="Optional commit SHA"
                        />
                      ),
                    }),
                    defineField({
                      key: 'spec.composePaths',
                      label: 'Compose Paths',
                      description: 'Compose files relative to repository root.',
                      render: (value, set) => (
                        <MonacoToArrayEditor
                          value={value}
                          helperText="# compose.yml"
                          language="string_list"
                          onChange={(v: string[] | undefined) =>
                            set((prev) => ({
                              spec: {
                                ...(prev.spec as any),
                                $type: 'Git',
                                composePaths: v ?? [],
                              } as any,
                            }))
                          }
                        />
                      ),
                    }),
                    defineField({
                      key: 'spec.additionalEnvFileFromRepo',
                      label: 'Additional Env Files',
                      description: 'Optional env files from the repository root.',
                      render: (value, set) => (
                        <MonacoToArrayEditor
                          value={value}
                          helperText="# .env.production"
                          language="string_list"
                          onChange={(v: string[] | undefined) =>
                            set((prev) => ({
                              spec: {
                                ...(prev.spec as any),
                                $type: 'Git',
                                additionalEnvFileFromRepo: v ?? [],
                              } as any,
                            }))
                          }
                        />
                      ),
                    }),
                  ],
                }),
              ]
            : currentStackSource === StackSource.WebEditor
              ? [
                  defineGroupField<StackInput>({
                    id: 'manual_stack_source',
                    label: 'Compose File',
                    items: [
                      defineField({
                        key: 'spec.composeFile',
                        label: 'Compose File',
                        required: true,
                        description: 'Define your docker compose YAML content.',
                        validate: (v) => (!v ? 'Compose file is required' : null),
                        render: (value, set) => (
                          <MonacoEditor
                            language="yaml"
                            filename="compose.yaml"
                            value={value ?? DEFAULT_STACK_FILE_CONTENTS}
                            onValueChange={(v) =>
                              set((prev) => ({
                                spec: {
                                  ...(prev.spec as any),
                                  $type: 'WebEditor',
                                  composeFile: v,
                                } as any,
                              }))
                            }
                          />
                        ),
                      }),
                    ],
                  }),
                ]
              : []),

          ...(currentStackSource
            ? [
                defineGroupField<StackInput>({
                  id: 'stack_environment',
                  label: 'Environment',
                  title: 'Environment',
                  description: 'Configure env file path and inline environment variables.',
                  items: [
                    defineField<StackInput, 'spec.envFilePath'>({
                      key: 'spec.envFilePath',
                      label: 'Env File Path',
                      description: 'Optional default env file path used at deploy time.',
                      render: (value, set) => (
                        <FieldInput
                          value={value}
                          onChange={(v) =>
                            set((prev) => ({
                              spec: {
                                ...prev.spec!,
                                envFilePath: v || null,
                              },
                            }))
                          }
                          placeholder="e.g. .env"
                        />
                      ),
                    }),
                    defineField<StackInput, 'spec.envVars'>({
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
                  ],
                }),
                defineField<StackInput, 'spec.registryId'>({
                  key: 'spec.registryId',
                  label: 'Registry',
                  required: true,
                  description: 'Select the registry to pull the images from.',
                  render: (val, set) => {
                    return (
                      <ResourceSelectorField
                        sourceType={LookupResourceType.Stack}
                        targetType={LookupResourceType.Registry}
                        sourceResourceId={id}
                        selected={val}
                        onSelect={(v: any) =>
                          set((prev) => ({
                            spec: {
                              ...prev.spec!,
                              registryId: v.id,
                            },
                          }))
                        }
                        placeholder="Select Registry"
                        className="sm:min-w-100"
                      />
                    );
                  },
                }),
                defineField<StackInput, 'spec.updateBehavior'>({
                  key: 'spec.updateBehavior',
                  label: 'Auto Update',
                  description: 'Choose how the platform handles new stack versions when they become available.',
                  render: (value, set) => {
                    return (
                      <div className="flex flex-col gap-2">
                        <ItemSelector
                          collection={update_behaviors}
                          value={value}
                          disabled={disabled}
                          onChange={(updateBehavior: StackUpdateBehavior) => {
                            set((prev) => ({
                              spec: {
                                ...prev.spec!,
                                updateBehavior: updateBehavior,
                              },
                            }));
                          }}
                        />
                      </div>
                    );
                  },
                }),
              ]
            : []),
        ],
      }),
      ...(currentStackSource
        ? {
            Advanced: defineSection<StackInput>({
              title: 'Advanced',
              items: [
                defineField<StackInput, 'spec.projectName'>({
                  key: 'spec.projectName',
                  label: 'Project Name',
                  description: 'Optional Docker Compose project name override.',
                  render: (value, set) => (
                    <FieldInput
                      value={value}
                      onChange={(v) =>
                        set((prev) => ({
                          spec: {
                            ...prev.spec!,
                            projectName: v || null,
                          },
                        }))
                      }
                      placeholder="Optional project name"
                    />
                  ),
                }),
                defineGroupField<StackInput>({
                  id: 'spec.preDeploy',
                  label: 'Pre Deploy',
                  title: 'Pre Deploy',
                  description:
                    "Execute a shell command before running docker compose up. The 'path' is relative to the Run Directory",
                  items: [
                    defineField({
                      key: 'spec.preDeploy.path',
                      label: 'Path',
                      render: (val, set) => (
                        <FieldInput
                          value={val}
                          onChange={(v) =>
                            set((prev) => ({
                              spec: {
                                ...prev.spec!,
                                preDeploy: {
                                  ...(prev.spec?.preDeploy ?? {}),
                                  path: v,
                                  commands: prev.spec?.preDeploy?.commands ?? [],
                                },
                              },
                            }))
                          }
                          placeholder="Command working directory"
                        />
                      ),
                    }),
                    defineField({
                      key: 'spec.preDeploy.commands',
                      label: 'Commands',
                      required: false,
                      render: (value, set) => (
                        <MonacoToArrayEditor
                          value={value}
                          helperText="# Add multiple commands on new lines"
                          language="string_list"
                          onChange={(v: string[] | undefined) =>
                            set((prev) => ({
                              spec: {
                                ...prev.spec!,
                                preDeploy: {
                                  ...(prev.spec?.preDeploy ?? {}),
                                  commands: v ?? [],
                                },
                              },
                            }))
                          }
                        />
                      ),
                    }),
                  ],
                }),
                defineGroupField<StackInput>({
                  id: 'spec.postDeploy',
                  label: 'Post Deploy',
                  title: 'Post Deploy',
                  description:
                    "Execute a shell command after running docker compose up. The 'path' is relative to the Run Directory",
                  items: [
                    defineField({
                      key: 'spec.postDeploy.path',
                      label: 'Path',
                      render: (val, set) => (
                        <FieldInput
                          value={val}
                          onChange={(v) =>
                            set((prev) => ({
                              spec: {
                                ...prev.spec!,
                                postDeploy: {
                                  ...(prev.spec?.postDeploy ?? {}),
                                  path: v,
                                  commands: prev.spec?.postDeploy?.commands ?? [],
                                },
                              },
                            }))
                          }
                          placeholder="Command working directory"
                        />
                      ),
                    }),
                    defineField({
                      key: 'spec.postDeploy.commands',
                      label: 'Commands',
                      required: false,
                      render: (value, set) => (
                        <MonacoToArrayEditor
                          value={value}
                          helperText="# Add multiple commands on new lines"
                          language="string_list"
                          onChange={(v: string[] | undefined) =>
                            set((prev) => ({
                              spec: {
                                ...prev.spec!,
                                postDeploy: {
                                  ...(prev.spec?.postDeploy ?? {}),
                                  commands: v ?? [],
                                },
                              },
                            }))
                          }
                        />
                      ),
                    }),
                  ],
                }),

                defineField({
                  key: 'spec.destroyBeforeDeploy',
                  label: 'Destroy',
                  description: `Ensure 'docker compose down' is run before redeploying the Stack.`,
                  required: false,
                  render: (value, set) => (
                    <FieldSwitch
                      checked={value ?? false}
                      id="spec.destroyBeforeDeploy"
                      onChange={(value) =>
                        set((prev) => ({
                          spec: {
                            ...prev.spec!,
                            destroyBeforeDeploy: value,
                          },
                        }))
                      }
                    />
                  ),
                }),
              ],
            }),
          }
        : {}),
    }),
    [disabled, mode, id, currentStackSource],
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
      disabled={disabled}
      draftKey={`stack:${id ?? 'new'}`}
      draftVersion={1}
    />
  );
};

const DEFAULT_STACK_FILE_CONTENTS = `## Add your compose file here
services:
  hello_world:
    image: hello-world
    # networks:
    #   - default
    # ports:
    #   - 3000:3000
    # volumes:
    #   - data:/data

# networks:
#   default: {}

# volumes:
#   data:
`;
