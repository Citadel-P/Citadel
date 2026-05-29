import {
  RegistryType,
  RegistryConfigurationGitHubRegistry,
  RegistryConfigurationCustomRegistry,
  RegistryConfigurationDockerHubRegistry,
  RegistryStatus,
  CreateRegistryInput,
  PatchRegistryInput,
} from '@/api/generated/api.types';
import {
  FormShell,
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldSwitch,
  FieldTextArea,
  ItemSelector,
  InputGroupField,
} from '@/components/custom/form-builder';
import { Constants } from '@/lib/constants';
import { useState, useMemo } from 'react';
import { useMutate, useSaveResource } from '@/lib/hooks';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Badge } from '@/components/ui/badge';
import { useParams } from 'react-router';
import { Globe, MoveUpRight } from 'lucide-react';
import { DockerIcon, GitHubIcon } from '@/lib/icons';
import { useQueryClient } from '@tanstack/react-query';

const registryInfo = {
  DockerHub: {
    icon: DockerIcon,
    label: 'DockerHub',
    description: 'Private DockerHub registry',
  },
  GitHub: {
    icon: GitHubIcon,
    label: 'GitHub',
    description: 'GitHub Container Registry (GHCR)',
  },
  Custom: {
    icon: Globe,
    label: 'Custom',
    description: 'Define your own OCI-compatible registry',
  },
} as const;

const registry_status = {
  [RegistryStatus.Active]: {
    label: RegistryStatus.Active,
    description: 'Available for selection and normal use.',
  },
  [RegistryStatus.Deprecated]: {
    label: RegistryStatus.Deprecated,
    description: 'Available for selection, but shown with a warning.',
  },
  [RegistryStatus.Disabled]: {
    label: RegistryStatus.Disabled,
    description: 'Hidden from selection and cannot be used.',
  },
};

const RegistryTypeSelector = ({ value, onChange, disabled }: any) => {
  const selected = registryInfo[value as keyof typeof registryInfo];

  return (
    <Select value={value} onValueChange={onChange} disabled={disabled}>
      <SelectTrigger className="w-full max-w-100">
        <SelectValue>
          {selected ? (
            <div className="flex items-center gap-2">
              <selected.icon className="w-4 h-4" />
              <span>{selected.label}</span>
            </div>
          ) : (
            'Select registry type'
          )}
        </SelectValue>
      </SelectTrigger>

      <SelectContent className="bg-background">
        {Object.entries(registryInfo).map(([key, info]) => (
          <SelectItem key={key} value={key}>
            <div className="flex items-center gap-2">
              <info.icon className="w-4 h-4" />
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

const HelperLink = ({ href, info }: { href: string; info: string }) => (
  <a className="underline decoration-dotted hover:text-primary" href={href} target="_blank" rel="noreferrer">
    <span className="flex flex-row gap-1 justify-baseline items-center">
      {info} <MoveUpRight className="h-3.5 w-3.5" />
    </span>
  </a>
);
type RegistryInput = CreateRegistryInput | PatchRegistryInput;

export const RegistryForm = ({
  mode,
  resource,
  disabled,
}: {
  mode: 'add' | 'edit';
  resource?: RegistryInput;
  disabled?: boolean;
}) => {
  const id = useParams().id;
  const queryClient = useQueryClient();
  const [update, setUpdate] = useState<Partial<RegistryInput>>({});

  const { mutateAsync: createRegistry } = useMutate('createRegistry');
  const { mutateAsync: updateRegistry } = useMutate('updateRegistry');

  const { save: handleSave, isPending } = useSaveResource<RegistryInput, any>({
    mode,
    basePath: 'registries',
    entityName: 'Registry',
    onCreate: (payload) => createRegistry({ data: payload as CreateRegistryInput }),
    onUpdate: (payload) => updateRegistry({ id: id!, data: payload as PatchRegistryInput }),
    onRefresh: () => {
      localStorage.removeItem(`Registry:${id ?? 'new'}`);
      queryClient.invalidateQueries({ queryKey: ['getRegistryConfig', { id }] });
    },
  });

  const original = resource ?? ({} as RegistryInput);
  const provider = update.configuration?.$type ?? resource?.configuration?.$type ?? RegistryType.DockerHub;
  const isCustomAuthEnabled =
    ((update.configuration as RegistryConfigurationCustomRegistry)?.authEnabled ??
      (original.configuration as RegistryConfigurationCustomRegistry)?.authEnabled) === true;
  const isGhcrAuthEnabled =
    ((update.configuration as RegistryConfigurationGitHubRegistry)?.ghcrAuthEnabled ??
      (original.configuration as RegistryConfigurationGitHubRegistry)?.ghcrAuthEnabled) === true;

  const schema = useMemo(
    () => ({
      '': defineSection<RegistryInput>({
        title: '',
        items: [
          defineField({
            key: 'configuration.$type',
            label: 'Provider',
            required: true,
            disabled: mode === 'edit',
            render: (value, set) => (
              <RegistryTypeSelector
                value={value ?? RegistryType.DockerHub}
                disabled={mode === 'edit'}
                onChange={(v: RegistryType) =>
                  set(() => ({
                    configuration: {
                      $type: v,
                    } as any,
                  }))
                }
              />
            ),
          }),
          ...(mode === 'add'
            ? [
                defineGroupField<RegistryInput>({
                  id: 'details',
                  label: 'Details',
                  items: [
                    defineField({
                      key: 'name',
                      label: 'Name',
                      description: 'Provide a unique name to identify this registry.',
                      required: true,
                      validate: (v) =>
                        !new RegExp(Constants.validNameIdentifier).test(v) ? 'Invalid name format' : null,
                      render: (value, set) => (
                        <FieldInput value={value ?? ''} onChange={(v) => set({ name: v })} placeholder="my-registry" />
                      ),
                    }),
                    defineField({
                      key: 'description',
                      label: 'Description',
                      required: false,
                      description: 'Optional notes to describe the registry’s purpose or usage.',
                      render: (val, set) => <FieldTextArea value={val} onChange={(v) => set({ description: v })} />,
                    }),
                  ],
                }),
              ]
            : []),

          defineField({
            label: 'Status',
            key: 'status',
            description: 'Choose the current state of the registry.',
            render: (val, set) => (
              <ItemSelector
                collection={registry_status}
                value={val}
                onChange={(v: RegistryStatus) => set({ status: v })}
              />
            ),
          }),
        ],
      }),

      ...(provider === RegistryType.GitHub
        ? {
            GitHub: defineSection<RegistryInput>({
              title: 'GitHub',
              items: [
                defineGroupField<RegistryInput>({
                  id: 'ghcr-auth',
                  label: 'Settings',
                  items: [
                    defineField({
                      key: 'configuration.nameSpace',
                      label: 'Namespace',
                      disabled: false,
                      required: true,
                      validate: (v) => {
                        const namespace = v?.replace('ghcr.io/', '') ?? '';
                        return !new RegExp(Constants.validNameIdentifier).test(namespace)
                          ? 'Invalid namespace (use a-z, 0-9)'
                          : null;
                      },
                      description: 'The GitHub account (User or Organization) that owns the container images.',
                      render: (value, set) => {
                        return (
                          <InputGroupField
                            value={value}
                            onChange={(value) =>
                              set((prev) => ({
                                configuration: {
                                  $type: 'GitHub',
                                  ...((prev.configuration ?? {}) as RegistryConfigurationGitHubRegistry),
                                  nameSpace: value,
                                } satisfies RegistryConfigurationGitHubRegistry,
                              }))
                            }
                            prefixPlaceholder="ghcr.io/"
                            suffixPlaceholder="e.g. jellyfin"
                          />
                        );
                      },
                    }),
                    defineField({
                      key: 'configuration.ghcrAuthEnabled',
                      label: 'Authentication',
                      description:
                        'Authentication is required for private images. For public images, providing authentication is recommended to avoid GitHub rate limits.',
                      render: (value, set) => (
                        <FieldSwitch
                          checked={value ?? false}
                          id="configuration.authEnabled"
                          onChange={(value) =>
                            set((prev) => ({
                              configuration: {
                                $type: 'GitHub',
                                ...((prev.configuration ?? {}) as RegistryConfigurationGitHubRegistry),
                                ghcrAuthEnabled: value,
                              } satisfies RegistryConfigurationGitHubRegistry,
                            }))
                          }
                        />
                      ),
                    }),
                  ],
                }),

                /* CREDENTIALS GROUP — only if enabled */
                ...(isGhcrAuthEnabled
                  ? [
                      defineField<RegistryInput, 'configuration.pat'>({
                        key: 'configuration.pat',
                        label: 'PAT',
                        required: true,
                        validate: (v) => (!v || v.length < 10 ? 'PAT must be at least 10 chars' : null),
                        description: (
                          <div className="flex flex-row flex-wrap text-sm gap-1 text-muted-foreground">
                            Provide a Personal Access Token with the <Badge variant="secondary">read:packages</Badge>{' '}
                            scope. More info in the{' '}
                            <HelperLink
                              href="https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens#creating-a-personal-access-token-classic"
                              info="GitHub documentation"
                            />
                          </div>
                        ),
                        render: (value, set) => (
                          <FieldInput
                            type="password"
                            value={value ?? ''}
                            onChange={(v) =>
                              set((prev) => ({
                                configuration: {
                                  $type: 'GitHub',
                                  ...((prev.configuration as RegistryConfigurationGitHubRegistry) ?? {}),
                                  pat: v,
                                } satisfies RegistryConfigurationGitHubRegistry,
                              }))
                            }
                          />
                        ),
                      }),
                    ]
                  : []),
              ],
            }),
          }
        : {}),

      ...(provider === RegistryType.DockerHub
        ? {
            DockerHub: defineSection<RegistryInput>({
              title: 'DockerHub',
              items: [
                defineField({
                  key: 'configuration.userName',
                  label: 'Username',
                  required: true,
                  validate: (v) => (!v || v.length < 3 ? 'DockerHub Username too short' : null),
                  render: (value, set) => (
                    <FieldInput
                      value={value ?? ''}
                      onChange={(v) =>
                        set((prev) => ({
                          configuration: {
                            $type: 'DockerHub',
                            ...((prev.configuration as RegistryConfigurationDockerHubRegistry) ?? {}),
                            userName: v,
                          } satisfies RegistryConfigurationDockerHubRegistry,
                        }))
                      }
                    />
                  ),
                }),

                defineField({
                  key: 'configuration.pat',
                  label: 'PAT',
                  required: true,
                  validate: (v) => (!v || v.length < 10 ? 'PAT must be at least 10 chars' : null),
                  description: (
                    <div className="flex flex-row text-sm text-muted-foreground gap-1">
                      To create a DockerHub personal access token, follow the{' '}
                      <HelperLink
                        href="https://docs.docker.com/security/for-developers/access-tokens/"
                        info="official DockerHub guide"
                      />
                    </div>
                  ),
                  render: (value, set) => (
                    <FieldInput
                      type="password"
                      value={value ?? ''}
                      onChange={(v) =>
                        set((prev) => ({
                          configuration: {
                            $type: 'DockerHub',
                            ...((prev.configuration as RegistryConfigurationDockerHubRegistry) ?? {}),
                            pat: v,
                          } satisfies RegistryConfigurationDockerHubRegistry,
                        }))
                      }
                    />
                  ),
                }),
              ],
            }),
          }
        : {}),

      ...(provider === RegistryType.Custom
        ? {
            Custom: defineSection<RegistryInput>({
              title: 'Custom Settings',
              items: [
                defineGroupField<RegistryInput>({
                  id: 'custom-general',
                  label: 'General',
                  items: [
                    defineField({
                      key: 'registryHost',
                      label: 'Registry Host',
                      required: true,
                      description: (
                        <div className="flex flex-row flex-wrap text-sm gap-1 text-muted-foreground">
                          Host or IP of the Docker registry. Use <Badge variant="secondary">host:port</Badge> format
                          only — no protocol.
                        </div>
                      ),
                      validate: (v) => (!new RegExp(Constants.validHostOrIp).test(v) ? 'Invalid host' : null),
                      render: (value, set) => (
                        <FieldInput
                          value={value ?? ''}
                          placeholder="myregistry.example"
                          onChange={(v) => set({ registryHost: v, configuration: { $type: 'Custom' } })}
                        />
                      ),
                    }),

                    defineField({
                      key: 'configuration.authEnabled',
                      label: 'Authentication',
                      description: 'Enable this option if you need to specify credentials to connect to this registry.',
                      render: (value, set) => (
                        <FieldSwitch
                          checked={value ?? false}
                          id="configuration.authEnabled"
                          onChange={(value) =>
                            set((prev) => ({
                              configuration: {
                                $type: 'Custom',
                                ...((prev.configuration ?? {}) as RegistryConfigurationCustomRegistry),
                                authEnabled: value,
                              } satisfies RegistryConfigurationCustomRegistry,
                            }))
                          }
                        />
                      ),
                    }),
                  ],
                }),

                /* CREDENTIALS GROUP — only if enabled */
                ...(isCustomAuthEnabled
                  ? [
                      defineGroupField<RegistryInput>({
                        id: 'custom-auth',
                        label: 'Credentials',
                        items: [
                          defineField({
                            key: 'configuration.userName',
                            label: 'Username',
                            required: true,
                            validate: (v) => (!v || v.length < 3 ? 'Username too short' : null),
                            render: (value, set) => (
                              <FieldInput
                                value={value ?? ''}
                                placeholder="username"
                                onChange={(v) =>
                                  set((prev) => ({
                                    configuration: {
                                      $type: 'Custom',
                                      ...((prev.configuration ?? {}) as RegistryConfigurationCustomRegistry),
                                      userName: v,
                                    } satisfies RegistryConfigurationCustomRegistry,
                                  }))
                                }
                              />
                            ),
                          }),

                          defineField({
                            key: 'configuration.password',
                            label: 'Password',
                            required: true,
                            validate: (v) => (!v || v.length < 4 ? 'Password too short' : null),
                            render: (value, set) => (
                              <FieldInput
                                type="password"
                                value={value ?? ''}
                                placeholder="password"
                                onChange={(v) =>
                                  set((prev) => ({
                                    configuration: {
                                      $type: 'Custom',
                                      ...((prev.configuration ?? {}) as RegistryConfigurationCustomRegistry),
                                      password: v,
                                    },
                                  }))
                                }
                              />
                            ),
                          }),
                        ],
                      }),
                    ]
                  : []),
              ],
            }),
          }
        : {}),
    }),
    [mode, provider, isGhcrAuthEnabled, isCustomAuthEnabled],
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
      draftKey={`registry:${id ?? 'new'}`}
      draftVersion={1}
    />
  );
};
