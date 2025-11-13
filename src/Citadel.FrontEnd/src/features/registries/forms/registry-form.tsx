import { useState } from 'react';
import { toast } from 'sonner';
import {
  RegistryInput,
  RegistryWithConfigView,
  RegistryType,
  GhcrAccountType,
  RegistryConfigurationBaseGitHubRegistry,
  RegistryConfigurationBaseDockerHubRegistry,
} from '@/api/generated/api.types';
import { useMutate } from '@/lib/hooks';
import {
  FormBuilder,
  FieldSection,
  ConfigInput,
  FieldRenderer,
  defineConfigComponent,
} from '@/components/custom/form-builder';
import { ResourceFormProps } from '@/pages/types';
import { Badge } from '@/components/ui/badge';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

const RegistryTypeSelector = ({
  selected,
  onSelect,
  disabled,
}: {
  selected: RegistryType;
  onSelect: (type: RegistryType) => void;
  disabled?: boolean;
}) => (
  <Select value={selected} onValueChange={onSelect} disabled={disabled}>
    <SelectTrigger className="w-full max-w-[400px]" disabled={disabled}>
      <SelectValue placeholder="Select registry type" />
    </SelectTrigger>
    <SelectContent className="bg-background">
      <SelectItem value={RegistryType.DockerHub}>DockerHub</SelectItem>
      <SelectItem value={RegistryType.GitHub}>GitHub</SelectItem>
    </SelectContent>
  </Select>
);

const GhcrAccountTypeSelector = ({
  selected,
  onSelect,
  disabled,
}: {
  selected?: GhcrAccountType;
  onSelect: (type: GhcrAccountType) => void;
  disabled?: boolean;
}) => (
  <FieldRenderer label="Account Type">
    <Select value={selected} onValueChange={onSelect} disabled={disabled}>
      <SelectTrigger className="w-full max-w-[400px]" disabled={disabled}>
        <SelectValue placeholder="Select GitHub account type" />
      </SelectTrigger>
      <SelectContent className="bg-background">
        <SelectItem value={GhcrAccountType.User}>User</SelectItem>
        <SelectItem value={GhcrAccountType.Organization}>Organization</SelectItem>
      </SelectContent>
    </Select>
  </FieldRenderer>
);

const registryTypeSelector: FieldSection<RegistryInput> = {
  label: 'Provider',
  description: 'Select the registry provider to connect to.',
  components: {
    type: (value, set) => (
      <RegistryTypeSelector
        selected={value ?? RegistryType.DockerHub}
        onSelect={(type) => set({ type, configuration: {} as any })}
      />
    ),
  },
};

function defineRegistryConfiguration(map: Record<string, FieldSection<RegistryInput>>) {
  return map;
}

const dockerHubConfiguration = defineRegistryConfiguration({
  name: defineConfigComponent<RegistryInput>({
    label: 'Name',
    description: 'Unique name for this registry.',
    components: {
      name: (value, set) => (
        <ConfigInput label="" value={value} placeholder="my-dockerhub" onChange={(v) => set({ name: v })} />
      ),
    },
  }),

  userName: defineConfigComponent<RegistryInput>({
    label: 'Username',
    description: 'DockerHub username.',
    components: {
      configuration: (value, set) => {
        const config = value as RegistryConfigurationBaseDockerHubRegistry;
        return (
          <ConfigInput
            label=""
            value={config?.userName ?? ''}
            onChange={(v) =>
              set({
                configuration: { ...(config ?? {}), userName: v, $type: 'DockerHub' },
              })
            }
          />
        );
      },
    },
  }),

  pat: defineConfigComponent<RegistryInput>({
    label: 'PAT',
    description: (
      <>
        To create a DockerHub access token, follow the{' '}
        <a
          className="underline"
          href="https://docs.docker.com/security/for-developers/access-tokens/"
          target="_blank"
          rel="noreferrer">
          official DockerHub guide
        </a>
        .
      </>
    ),
    components: {
      configuration: (value, set) => {
        const config = value as RegistryConfigurationBaseDockerHubRegistry;
        return (
          <ConfigInput
            label=""
            value={config?.pat ?? ''}
            onChange={(v) =>
              set({
                configuration: { ...(config ?? {}), pat: v, $type: 'DockerHub' },
              })
            }
          />
        );
      },
    },
  }),
});

const gitHubConfiguration = defineRegistryConfiguration({
  name: defineConfigComponent<RegistryInput>({
    label: 'Name',
    description: 'Unique name for this registry.',
    components: {
      name: (value, set) => (
        <ConfigInput label="" value={value} placeholder="my-ghcr-registry" onChange={(v) => set({ name: v })} />
      ),
    },
  }),

  account: defineConfigComponent<RegistryInput>({
    label: 'Account',
    description: 'GitHub account information',
    components: {
      configuration: (value, set) => {
        const config = value as RegistryConfigurationBaseGitHubRegistry;
        return (
          <>
            <GhcrAccountTypeSelector
              selected={config?.type ?? GhcrAccountType.User}
              onSelect={(v) =>
                set({
                  configuration: {
                    ...(config ?? {}),
                    type: v,
                    $type: 'GitHub',
                  },
                })
              }
            />
            <ConfigInput
              label="Account name"
              value={config?.name ?? ''}
              onChange={(v) =>
                set({
                  configuration: {
                    ...(config ?? {}),
                    name: v,
                    $type: 'GitHub',
                  },
                })
              }
            />
          </>
        );
      },
    },
  }),

  pat: defineConfigComponent<RegistryInput>({
    label: 'PAT',
    description: (
      <>
        Provide a Personal Access Token with the <Badge variant="secondary">read:packages</Badge> scope. Refer to{' '}
        <a
          className="underline"
          target="_blank"
          rel="noreferrer"
          href="https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens#creating-a-personal-access-token-classic">
          GitHub documentation
        </a>{' '}
        for more details.
      </>
    ),
    components: {
      configuration: (value, set) => {
        const config = value as RegistryConfigurationBaseGitHubRegistry;
        return (
          <ConfigInput
            label=""
            value={config?.pat ?? ''}
            onChange={(v) =>
              set({
                configuration: { ...(config ?? {}), pat: v, $type: 'GitHub' },
              })
            }
          />
        );
      },
    },
  }),
});

export const RegistryForm: React.FC<ResourceFormProps<RegistryWithConfigView>> = ({ mode, resource }) => {
  const [update, setUpdate] = useState<Partial<RegistryInput>>({});
  const { mutate: createRegistry, isPending: creating } = useMutate('createRegistry');
  const { mutate: updateRegistry, isPending: updating } = useMutate('updateRegistry');

  const isEdit = mode === 'edit';
  const original = (resource as RegistryInput) ?? ({} as RegistryInput);
  const providerType = (update.type ?? resource?.type ?? RegistryType.DockerHub) as RegistryType;

  const handleSave = async () => {
    const payload = { ...original, ...update } as RegistryInput;
    if (isEdit) await updateRegistry({ id: resource!.id, data: payload });
    else await createRegistry({ data: payload });
    toast.success(`Registry "${payload.name}" ${isEdit ? 'updated' : 'created'} successfully`);
  };

  const components =
    providerType === RegistryType.GitHub
      ? {
          '': [registryTypeSelector],
          GitHub: [gitHubConfiguration.name, gitHubConfiguration.account, gitHubConfiguration.pat],
        }
      : {
          General: [registryTypeSelector],
          DockerHub: [dockerHubConfiguration.name, dockerHubConfiguration.userName, dockerHubConfiguration.pat],
        };

  return (
    <FormBuilder
      original={original}
      update={update}
      set={setUpdate}
      disabled={creating || updating}
      onSave={handleSave}
      components={components}
    />
  );
};
