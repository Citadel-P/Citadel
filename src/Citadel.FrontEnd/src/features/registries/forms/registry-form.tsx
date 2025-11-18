import {
  RegistryType,
  RegistryInput,
  GhcrAccountType,
  RegistryConfigurationBaseGitHubRegistry,
  RegistryConfigurationBaseDockerHubRegistry,
} from '@/api/generated/api.types';
import { FormShell, defineField, defineGroupField, defineSection, FieldInput } from '@/components/custom/form-builder';
import { Constants } from '@/lib/constants';
import { useState, useMemo } from 'react';
import { useMutate } from '@/lib/hooks';
import { toast } from 'sonner';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Badge } from '@/components/ui/badge';
import { useParams, useNavigate } from 'react-router';
import { MoveUpRight } from 'lucide-react';

const RegistryTypeSelector = ({ value, onChange, disabled }: any) => (
  <Select value={value} onValueChange={onChange} disabled={disabled}>
    <SelectTrigger className="w-full max-w-[400px]">
      <SelectValue placeholder="Select registry type" />
    </SelectTrigger>
    <SelectContent className="bg-background">
      <SelectItem value="DockerHub">DockerHub</SelectItem>
      <SelectItem value="GitHub">GitHub</SelectItem>
    </SelectContent>
  </Select>
);

const GhcrAccountTypeSelector = ({ value, onChange, disabled }: any) => (
  <Select value={value} onValueChange={onChange} disabled={disabled}>
    <SelectTrigger className="w-full max-w-[400px]">
      <SelectValue placeholder="Select GitHub account type" />
    </SelectTrigger>
    <SelectContent className="bg-background">
      <SelectItem value={GhcrAccountType.User}>{GhcrAccountType.User}</SelectItem>
      <SelectItem value={GhcrAccountType.Organization}>{GhcrAccountType.Organization}</SelectItem>
    </SelectContent>
  </Select>
);

const HelperLink = ({ href, info }: { href: string; info: string }) => {
  return (
    <a className="underline decoration-dotted hover:text-primary" href={href} target="_blank" rel="noreferrer">
      <span className="flex flex-row gap-1 justify-baseline items-center">
        {info} <MoveUpRight className="h-3.5 w-3.5" />
      </span>
    </a>
  );
};

export const RegistryForm = ({ mode, resource }: { mode: 'add' | 'edit'; resource?: RegistryInput }) => {
  const id = useParams().id;
  const navigate = useNavigate();
  const [update, setUpdate] = useState<Partial<RegistryInput>>({});
  const [isPending, setIsPending] = useState(false);

  const { mutateAsync: createRegistry } = useMutate('createRegistry');
  const { mutateAsync: updateRegistry } = useMutate('updateRegistry');

  const original = resource ?? ({} as RegistryInput);
  const provider = update.type ?? resource?.type ?? RegistryType.DockerHub;

  const handleSave = async (payload: RegistryInput) => {
    setIsPending(true);
    try {
      if (mode === 'edit') await updateRegistry({ id, data: payload });
      else await createRegistry({ data: payload });

      toast.success(`Registry "${payload.name}" saved successfully`);
      navigate('/registries');
    } finally {
      setIsPending(false);
    }
  };

  const schema = useMemo(
    () => ({
      '': defineSection<RegistryInput>({
        title: '',
        items: [
          defineField({
            key: 'type',
            label: 'Provider',
            required: true,
            disabled: mode === 'edit',
            render: (value, set) => (
              <RegistryTypeSelector
                value={value ?? RegistryType.DockerHub}
                onChange={(v: RegistryType) =>
                  set(() => ({
                    type: v,
                    configuration: undefined, // Reset the form configuration on value change
                  }))
                }
              />
            ),
          }),

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

      [provider === RegistryType.GitHub ? 'GitHub' : 'DockerHub']:
        provider === RegistryType.GitHub
          ? defineSection<RegistryInput>({
              title: 'GitHub',
              items: [
                defineGroupField<RegistryInput>({
                  id: 'account-info',
                  label: 'Account Info',
                  fields: [
                    defineField({
                      key: 'configuration.type',
                      label: 'Account Type',
                      description: 'Select your account type',
                      required: true,
                      validate: (v) => (!v || v.length < 3 ? 'Account name too short' : null),
                      render: (value, set) => (
                        <GhcrAccountTypeSelector
                          value={value ?? GhcrAccountType.User}
                          onChange={(v: GhcrAccountType) =>
                            set((prev) => ({
                              configuration: {
                                ...((prev.configuration as RegistryConfigurationBaseGitHubRegistry) ?? {}),
                                type: v,
                                $type: 'GitHub',
                              } satisfies RegistryConfigurationBaseGitHubRegistry,
                            }))
                          }
                        />
                      ),
                    }),

                    defineField({
                      key: 'configuration.name',
                      label:
                        (update.configuration as RegistryConfigurationBaseGitHubRegistry)?.type ===
                        GhcrAccountType.Organization
                          ? 'Organization Name'
                          : 'User Name',
                      required: true,
                      validate: (v) => (!v || v.length < 3 ? 'Account name too short' : null),
                      render: (value, set) => (
                        <FieldInput
                          value={value ?? ''}
                          onChange={(v) =>
                            set((prev) => ({
                              configuration: {
                                ...((prev.configuration as RegistryConfigurationBaseGitHubRegistry) ?? {}),
                                name: v,
                                $type: 'GitHub',
                              } satisfies RegistryConfigurationBaseGitHubRegistry,
                            }))
                          }
                        />
                      ),
                    }),
                  ],
                }),
                defineField({
                  key: 'configuration.pat',
                  label: 'PAT',
                  description: (
                    <div className="flex flex-row flex-wrap text-sm gap-1 text-muted-foreground">
                      Provide a Personal Access Token with the <Badge variant="secondary">read:packages</Badge> scope.
                      More info in the
                      <HelperLink
                        href="https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens#creating-a-personal-access-token-classic"
                        info="GitHub documentation"
                      />
                    </div>
                  ),
                  required: true,
                  validate: (v) => (!v || v.length < 10 ? 'PAT must be at least 10 chars' : null),
                  render: (value, set) => (
                    <FieldInput
                      type="password"
                      value={value ?? ''}
                      onChange={(v) =>
                        set((prev) => ({
                          configuration: {
                            ...((prev.configuration as RegistryConfigurationBaseGitHubRegistry) ?? {}),
                            pat: v,
                            $type: 'GitHub',
                          } satisfies RegistryConfigurationBaseGitHubRegistry,
                        }))
                      }
                    />
                  ),
                }),
              ],
            })
          : defineSection<RegistryInput>({
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
                            ...((prev.configuration as RegistryConfigurationBaseDockerHubRegistry) ?? {}),
                            userName: v,
                            $type: 'DockerHub',
                          },
                        }))
                      }
                    />
                  ),
                }),

                defineField({
                  key: 'configuration.pat',
                  label: 'PAT',
                  description: (
                    <div className="flex flex-row text-sm text-muted-foreground gap-1">
                      To create a DockerHub personal access token, follow the{' '}
                      <HelperLink
                        href="https://docs.docker.com/security/for-developers/access-tokens/"
                        info="official DockerHub guide"
                      />
                    </div>
                  ),
                  required: true,
                  validate: (v) => (!v || v.length < 10 ? 'PAT must be at least 10 chars' : null),
                  render: (value, set) => (
                    <FieldInput
                      type="password"
                      value={value ?? ''}
                      onChange={(v) =>
                        set((prev) => ({
                          configuration: {
                            ...((prev.configuration as RegistryConfigurationBaseDockerHubRegistry) ?? {}),
                            pat: v,
                            $type: 'DockerHub',
                          },
                        }))
                      }
                    />
                  ),
                }),
              ],
            }),
    }),
    [mode, provider, update.configuration],
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
      draftKey={`registry:${id ?? 'new'}`}
      draftVersion={1}
    />
  );
};
