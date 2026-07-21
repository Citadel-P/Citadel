import {
  BuildAgentPoolInput,
  BuildAgentPoolProvider,
  BuildAgentPoolProviderSpecAwsEc2BuildAgentPoolProviderSpec,
  BuildAgentPoolProviderSpecSelfManagedVmBuildAgentPoolProviderSpec,
  BuildAgentPoolView,
  CpuArchitecture,
  UpdateBuildAgentPoolInput,
} from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import {
  FieldInput,
  FieldSelect,
  FieldSwitch,
  FieldTextArea,
  FormShell,
  defineField,
  defineGroupField,
  defineRowField,
  defineSection,
} from '@/components/custom/form-builder';
import { StateIndicator } from '@/components/custom/state-indicator';
import { ActivitiesTab } from '@/features/activities';
import { ResourceHeaderTagsEditor, ResourceTagSelector } from '@/features/tags/components';
import { Constants } from '@/lib/constants';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { hasCapability } from '@/lib/resource-capabilities';
import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useMemo, useState } from 'react';
import { useParams } from 'react-router';
import { BuildPoolInfoActions, invalidateBuildPoolQueries } from './actions';

type BuildPoolInput = BuildAgentPoolInput | UpdateBuildAgentPoolInput;
type BuildPoolFormResource = BuildAgentPoolView & RequiredFormFields;
type AwsEc2ProviderSpec = BuildAgentPoolProviderSpecAwsEc2BuildAgentPoolProviderSpec & { $type: 'AwsEc2' };
type SelfManagedVmProviderSpec = BuildAgentPoolProviderSpecSelfManagedVmBuildAgentPoolProviderSpec & {
  $type: 'SelfManagedVm';
};
type BuildPoolProviderSpec = AwsEc2ProviderSpec | SelfManagedVmProviderSpec;

const defaultAwsSpec: AwsEc2ProviderSpec = {
  $type: 'AwsEc2',
  provider: BuildAgentPoolProvider.AwsEc2,
  region: '',
  instanceType: 'c5.2xlarge',
  architecture: CpuArchitecture.Amd64,
  amiId: '',
  rootVolumeSizeGb: 50,
  subnetId: '',
  securityGroupIds: [],
  instanceProfileName: null,
  assignPublicIp: false,
  awsCredentialSecretId: null,
  assumeRoleArn: null,
  keyPairName: null,
  tags: null,
};

const defaultSelfManagedVmSpec: SelfManagedVmProviderSpec = {
  $type: 'SelfManagedVm',
  provider: BuildAgentPoolProvider.SelfManagedVm,
  endpoint: '',
  architecture: CpuArchitecture.Amd64,
  maxWorkers: 1,
  registrationSecretId: null,
  labels: [],
};

const defaultBuildPool: BuildAgentPoolInput = {
  name: '',
  description: null,
  enabled: true,
  providerSpec: defaultAwsSpec,
  maxActiveBuilders: 1,
  queueTimeoutSeconds: 3600,
  provisioningTimeoutSeconds: 600,
  registrationTimeoutSeconds: 300,
  heartbeatTimeoutSeconds: 90,
  cleanupTimeoutSeconds: 600,
  maximumInstanceLifetimeSeconds: 7200,
  failureRetentionMinutes: 0,
  tagIds: [],
};

export const BuildPoolFormComponents: RequiredFormComponents<BuildPoolFormResource> = {
  AddForm: {
    Header: {
      title: 'Build Pool',
    },
    Content: () => <BuildPoolForm mode="add" />,
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }) => <StateIndicator value={(resource as BuildAgentPoolView).enabled} enableLabel />,
      ActionButtons: ({ resource }) => {
        const { edit: _edit, ...actions } = BuildPoolInfoActions;
        return <GenericActionBarButtons resource={resource} actions={Object.values(actions)} />;
      },
      Tags: ({ resource }) => (
        <ResourceHeaderTagsEditor
          resourceType="BuildAgentPool"
          resourceId={(resource as BuildAgentPoolView).id}
          tags={(resource as BuildAgentPoolView).tags}
          disabled={!hasCapability(resource, 'canWrite')}
        />
      ),
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }) => (
          <BuildPoolForm
            mode="edit"
            resource={resource as BuildAgentPoolView}
            disabled={!hasCapability(resource, 'canWrite')}
          />
        ),
      },
      {
        label: 'Activities',
        Content: ({ resource }) => (
          <ActivitiesTab resourceId={(resource as BuildAgentPoolView).id} resourceType="BuildAgentPool" />
        ),
      },
    ],
    useData(id: string) {
      const { data, isLoading } = useRead('getBuildAgentPool', { id });
      return { item: data?.data as BuildPoolFormResource | undefined, isLoading };
    },
  },
};

function BuildPoolForm({
  mode,
  resource,
  disabled,
}: {
  mode: 'add' | 'edit';
  resource?: BuildAgentPoolView;
  disabled?: boolean;
}) {
  const id = useParams().id;
  const queryClient = useQueryClient();
  const [update, setUpdate] = useState<Partial<BuildPoolInput>>({});
  const createPool = useMutate('createBuildAgentPool');
  const updatePool = useMutate('updateBuildAgentPool');

  const { save, isPending } = useSaveResource<BuildPoolInput, any>({
    mode,
    basePath: 'build-pools',
    entityName: 'Build Pool',
    onCreate: (payload) => createPool.mutateAsync({ data: normalizePoolPayload(payload, mode) as BuildAgentPoolInput }),
    onUpdate: (payload) =>
      updatePool.mutateAsync({ id: id!, data: normalizePoolPayload(payload, mode) as UpdateBuildAgentPoolInput }),
    onRefresh: () => {
      localStorage.removeItem(`BuildAgentPool:${id ?? 'new'}`);
      invalidateBuildPoolQueries(queryClient, id);
    },
    extractName: (payload, response) =>
      response?.data?.name ?? ('name' in payload ? payload.name : undefined) ?? resource?.name ?? 'Build Pool',
  });

  const original = useMemo<BuildPoolInput>(() => {
    if (!resource) return defaultBuildPool;

    return {
      description: resource.description,
      enabled: resource.enabled,
      providerSpec: normalizeProviderSpec(resource.providerSpec),
      maxActiveBuilders: resource.maxActiveBuilders,
      queueTimeoutSeconds: resource.queueTimeoutSeconds,
      provisioningTimeoutSeconds: resource.provisioningTimeoutSeconds,
      registrationTimeoutSeconds: resource.registrationTimeoutSeconds,
      heartbeatTimeoutSeconds: resource.heartbeatTimeoutSeconds,
      cleanupTimeoutSeconds: resource.cleanupTimeoutSeconds,
      maximumInstanceLifetimeSeconds: resource.maximumInstanceLifetimeSeconds,
      failureRetentionMinutes: resource.failureRetentionMinutes,
    };
  }, [resource]);

  const providerSpec = useMemo(
    () => normalizeProviderSpec(update.providerSpec ?? original.providerSpec),
    [original.providerSpec, update.providerSpec],
  );
  const provider = getProvider(providerSpec);
  const awsSpec = useMemo(() => normalizeAwsSpec(providerSpec), [providerSpec]);
  const vmSpec = useMemo(() => normalizeSelfManagedVmSpec(providerSpec), [providerSpec]);

  const setProviderSpec = useCallback(
    (spec: BuildPoolProviderSpec) => setUpdate((prev) => ({ ...prev, providerSpec: spec })),
    [],
  );
  const setAwsSpec = useCallback(
    (patch: Partial<AwsEc2ProviderSpec>) =>
      setProviderSpec({ ...normalizeAwsSpec(providerSpec), ...patch }),
    [providerSpec, setProviderSpec],
  );
  const setVmSpec = useCallback(
    (patch: Partial<SelfManagedVmProviderSpec>) =>
      setProviderSpec({ ...normalizeSelfManagedVmSpec(providerSpec), ...patch }),
    [providerSpec, setProviderSpec],
  );

  const schema = useMemo(
    () => ({
      General: defineSection<BuildPoolInput>({
        title: 'General',
        items: [
          ...(mode === 'add'
            ? [
                defineGroupField<BuildPoolInput>({
                  id: 'details',
                  label: 'Details',
                  items: [
                    defineField({
                      key: 'name',
                      label: 'Name',
                      required: true,
                      description: 'Internal name used when selecting this pool from a build project.',
                      validate: (v) =>
                        !new RegExp(Constants.validNameIdentifier).test(v ?? '') ? 'Invalid name format' : null,
                      render: (value, set) => (
                        <FieldInput value={value ?? ''} onChange={(name) => set({ name })} placeholder="build-pool" />
                      ),
                    }),
                    defineField({
                      key: 'description',
                      label: 'Description',
                      description: 'Optional notes about where these builders run.',
                      render: (value, set) => (
                        <FieldTextArea value={value ?? ''} onChange={(description) => set({ description })} />
                      ),
                    }),
                    defineField({
                      key: 'tagIds',
                      label: 'Tags',
                      description: 'Optional tags for filtering and access policies.',
                      render: (value, set) => (
                        <ResourceTagSelector value={value} onChange={(tagIds) => set({ tagIds })} />
                      ),
                    }),
                  ],
                }),
              ]
            : []),
          defineGroupField<BuildPoolInput>({
            id: 'state',
            label: 'State',
            items: [
              defineField({
                key: 'enabled',
                label: 'Enabled',
                description: 'Disabled pools cannot be selected for new build runs.',
                render: (value, set) => (
                  <FieldSwitch id="build-pool-enabled" checked={value !== false} onChange={(enabled) => set({ enabled })} />
                ),
              }),
            ],
          }),
        ],
      }),
      Provider: defineSection<BuildPoolInput>({
        title: 'Provider',
        items: [
          defineGroupField<BuildPoolInput>({
            id: 'provider',
            label: 'Provider',
            description: 'Choose the infrastructure type that supplies builders for this pool.',
            items: [
              defineField({
                key: 'providerSpec',
                label: 'Type',
                required: true,
                description: 'AWS EC2 provisions temporary builders. Self-managed VM uses pre-provisioned builder hosts.',
                render: () => (
                  <FieldSelect
                    value={provider}
                    disabled={disabled}
                    onChange={(nextProvider) =>
                      setProviderSpec(
                        nextProvider === BuildAgentPoolProvider.SelfManagedVm
                          ? { ...defaultSelfManagedVmSpec }
                          : { ...defaultAwsSpec },
                      )
                    }
                    options={[
                      { value: BuildAgentPoolProvider.AwsEc2, label: 'AWS EC2' },
                      { value: BuildAgentPoolProvider.SelfManagedVm, label: 'Self-managed VM / Static VM' },
                    ]}
                  />
                ),
              }),
            ],
          }),
          provider === BuildAgentPoolProvider.SelfManagedVm
            ? selfManagedVmFields(vmSpec, setVmSpec, disabled)
            : awsEc2Fields(awsSpec, setAwsSpec, disabled),
        ],
      }),
      Limits: defineSection<BuildPoolInput>({
        title: 'Limits',
        items: [
          defineGroupField<BuildPoolInput>({
            id: 'timeouts',
            label: 'Capacity and Timeouts',
            items: [
              defineNumberField('maxActiveBuilders', 'Max active builders', 'Maximum concurrently leased builders from this pool.'),
              defineNumberField('queueTimeoutSeconds', 'Queue timeout seconds', 'Maximum time a run can wait for a builder.'),
              defineNumberField('maximumInstanceLifetimeSeconds', 'Max lifetime seconds', 'Hard lifetime cap for temporary builders.'),
              defineNumberField('provisioningTimeoutSeconds', 'Provisioning timeout', 'Time allowed for builder provisioning.'),
              defineNumberField('registrationTimeoutSeconds', 'Registration timeout', 'Time allowed for the builder to connect back.'),
              defineNumberField('heartbeatTimeoutSeconds', 'Heartbeat timeout', 'Time without heartbeat before a builder is stale.'),
              defineNumberField('cleanupTimeoutSeconds', 'Cleanup timeout', 'Time allowed for builder cleanup.'),
              defineNumberField('failureRetentionMinutes', 'Failure retention minutes', 'How long failed builders may be retained for inspection.'),
            ],
          }),
        ],
      }),
    }),
    [awsSpec, disabled, mode, provider, setAwsSpec, setProviderSpec, setVmSpec, vmSpec],
  );

  return (
    <FormShell<BuildPoolInput>
      title=""
      schema={schema}
      original={original}
      update={update}
      setUpdate={setUpdate}
      onSave={save}
      disabled={disabled}
      pending={isPending}
      mode={mode}
      draftKey={`BuildAgentPool:${id ?? 'new'}`}
      draftVersion={resource?.rowVersion ?? 'new'}
    />
  );
}

function awsEc2Fields(
  awsSpec: AwsEc2ProviderSpec,
  setAwsSpec: (patch: Partial<AwsEc2ProviderSpec>) => void,
  disabled?: boolean,
) {
  return defineGroupField<BuildPoolInput>({
    id: 'aws',
    label: 'AWS EC2 Settings',
    description: 'Settings used when Citadel provisions temporary EC2 builder instances.',
    items: [
      defineField({
        key: 'providerSpec',
        label: 'Region',
        required: true,
        description: 'AWS region where builder instances are launched.',
        render: () => <FieldInput value={awsSpec.region} disabled={disabled} onChange={(region) => setAwsSpec({ region })} />,
      }),
      defineField({
        key: 'providerSpec',
        label: 'Instance type',
        required: true,
        description: 'EC2 instance shape used for each builder.',
        render: () => (
          <FieldInput
            value={awsSpec.instanceType}
            disabled={disabled}
            onChange={(instanceType) => setAwsSpec({ instanceType })}
            placeholder="c5.2xlarge"
          />
        ),
      }),
      defineField({
        key: 'providerSpec',
        label: 'Architecture',
        required: true,
        description: 'CPU architecture expected by the builder AMI.',
        render: () => (
          <FieldSelect
            value={awsSpec.architecture}
            disabled={disabled}
            onChange={(architecture) => setAwsSpec({ architecture: architecture as CpuArchitecture })}
            options={architectureOptions}
          />
        ),
      }),
      defineField({
        key: 'providerSpec',
        label: 'AMI ID',
        required: true,
        description: 'AMI that boots the Citadel builder runtime.',
        render: () => <FieldInput value={awsSpec.amiId} disabled={disabled} onChange={(amiId) => setAwsSpec({ amiId })} />,
      }),
      defineField({
        key: 'providerSpec',
        label: 'Root volume GB',
        required: true,
        description: 'Ephemeral disk size used for checkout, Docker cache, and image layers.',
        render: () => (
          <FieldInput
            type="number"
            value={awsSpec.rootVolumeSizeGb}
            disabled={disabled}
            onChange={(rootVolumeSizeGb) => setAwsSpec({ rootVolumeSizeGb })}
          />
        ),
      }),
      defineField({
        key: 'providerSpec',
        label: 'Subnet ID',
        required: true,
        description: 'Subnet where builders are launched.',
        render: () => <FieldInput value={awsSpec.subnetId} disabled={disabled} onChange={(subnetId) => setAwsSpec({ subnetId })} />,
      }),
      defineField({
        key: 'providerSpec',
        label: 'Security group IDs',
        required: true,
        description: 'Comma-separated security group IDs attached to builder instances.',
        render: () => (
          <FieldInput
            value={awsSpec.securityGroupIds.join(', ')}
            disabled={disabled}
            onChange={(value) => setAwsSpec({ securityGroupIds: splitCsv(value) })}
            placeholder="sg-123, sg-456"
          />
        ),
      }),
      defineRowField<BuildPoolInput>({
        id: 'aws-optional-access',
        className: 'flex flex-col gap-6',
        fields: [
          defineField({
            key: 'providerSpec',
            label: 'Instance profile',
            description: 'Optional IAM instance profile attached to builders.',
            render: () => (
              <FieldInput
                value={awsSpec.instanceProfileName ?? ''}
                disabled={disabled}
                onChange={(instanceProfileName) => setAwsSpec({ instanceProfileName: instanceProfileName || null })}
              />
            ),
          }),
          defineField({
            key: 'providerSpec',
            label: 'Assume role ARN',
            description: 'Optional AWS role Citadel assumes before provisioning builders.',
            render: () => (
              <FieldInput
                value={awsSpec.assumeRoleArn ?? ''}
                disabled={disabled}
                onChange={(assumeRoleArn) => setAwsSpec({ assumeRoleArn: assumeRoleArn || null })}
              />
            ),
          }),
        ],
      }),
      defineField({
        key: 'providerSpec',
        label: 'Assign public IP',
        description: 'Attach a public IP when the subnet does not provide outbound NAT.',
        render: () => (
          <FieldSwitch
            id="build-pool-public-ip"
            checked={awsSpec.assignPublicIp}
            onChange={(assignPublicIp) => setAwsSpec({ assignPublicIp })}
          />
        ),
      }),
      defineField({
        key: 'providerSpec',
        label: 'Key pair',
        description: 'Optional EC2 key pair for emergency access.',
        render: () => (
          <FieldInput
            value={awsSpec.keyPairName ?? ''}
            disabled={disabled}
            onChange={(keyPairName) => setAwsSpec({ keyPairName: keyPairName || null })}
          />
        ),
      }),
    ],
  });
}

function selfManagedVmFields(
  vmSpec: SelfManagedVmProviderSpec,
  setVmSpec: (patch: Partial<SelfManagedVmProviderSpec>) => void,
  disabled?: boolean,
) {
  return defineGroupField<BuildPoolInput>({
    id: 'self-managed-vm',
    label: 'Self-managed VM Settings',
    description: 'Settings for static builder hosts that are provisioned outside Citadel.',
    items: [
      defineField({
        key: 'providerSpec',
        label: 'Endpoint',
        required: true,
        description: 'Stable URL or address used by Citadel to identify or reach this builder pool.',
        render: () => (
          <FieldInput
            value={vmSpec.endpoint}
            disabled={disabled}
            onChange={(endpoint) => setVmSpec({ endpoint })}
            placeholder="https://builder-01.example.com"
          />
        ),
      }),
      defineField({
        key: 'providerSpec',
        label: 'Architecture',
        required: true,
        description: 'CPU architecture supported by the self-managed builders.',
        render: () => (
          <FieldSelect
            value={vmSpec.architecture}
            disabled={disabled}
            onChange={(architecture) => setVmSpec({ architecture: architecture as CpuArchitecture })}
            options={architectureOptions}
          />
        ),
      }),
      defineField({
        key: 'providerSpec',
        label: 'Max workers',
        required: true,
        description: 'Maximum workers available across this static builder pool.',
        render: () => (
          <FieldInput
            type="number"
            value={vmSpec.maxWorkers}
            disabled={disabled}
            onChange={(maxWorkers) => setVmSpec({ maxWorkers })}
          />
        ),
      }),
      defineRowField<BuildPoolInput>({
        id: 'self-managed-vm-optional',
        fields: [
          defineField({
            key: 'providerSpec',
            label: 'Registration secret ID',
            description: 'Optional Citadel secret ID used by future builder registration flows.',
            render: () => (
              <FieldInput
                value={vmSpec.registrationSecretId ?? ''}
                disabled={disabled}
                onChange={(registrationSecretId) => setVmSpec({ registrationSecretId: registrationSecretId || null })}
                placeholder="00000000-0000-0000-0000-000000000000"
              />
            ),
          }),
          defineField({
            key: 'providerSpec',
            label: 'Labels',
            description: 'Comma-separated labels used to describe builder capabilities.',
            render: () => (
              <FieldInput
                value={(vmSpec.labels ?? []).join(', ')}
                disabled={disabled}
                onChange={(value) => setVmSpec({ labels: splitCsv(value) })}
                placeholder="linux, docker, amd64"
              />
            ),
          }),
        ],
      }),
    ],
  });
}

function defineNumberField(key: keyof BuildAgentPoolInput, label: string, description: string) {
  return defineField<BuildPoolInput, any>({
    key: key as any,
    label,
    description,
    required: true,
    render: (value, set) => <FieldInput type="number" value={value ?? 0} onChange={(v) => set({ [key]: v } as any)} />,
  });
}

function normalizePoolPayload(payload: BuildPoolInput, mode: 'add' | 'edit') {
  const normalized = {
    ...payload,
    description: payload.description ?? null,
    providerSpec: normalizeProviderSpec(payload.providerSpec),
    maxActiveBuilders: Number(payload.maxActiveBuilders ?? 1),
    queueTimeoutSeconds: Number(payload.queueTimeoutSeconds ?? 3600),
    provisioningTimeoutSeconds: Number(payload.provisioningTimeoutSeconds ?? 600),
    registrationTimeoutSeconds: Number(payload.registrationTimeoutSeconds ?? 300),
    heartbeatTimeoutSeconds: Number(payload.heartbeatTimeoutSeconds ?? 90),
    cleanupTimeoutSeconds: Number(payload.cleanupTimeoutSeconds ?? 600),
    maximumInstanceLifetimeSeconds: Number(payload.maximumInstanceLifetimeSeconds ?? 7200),
    failureRetentionMinutes: Number(payload.failureRetentionMinutes ?? 0),
  };

  if (mode === 'edit') {
    delete (normalized as Partial<BuildAgentPoolInput>).name;
    delete (normalized as Partial<BuildAgentPoolInput>).tagIds;
  }

  return normalized;
}

function normalizeProviderSpec(value: unknown): BuildPoolProviderSpec {
  const provider = getProvider(value);
  return provider === BuildAgentPoolProvider.SelfManagedVm
    ? normalizeSelfManagedVmSpec(value)
    : normalizeAwsSpec(value);
}

function normalizeAwsSpec(value: unknown): AwsEc2ProviderSpec {
  const spec = { ...defaultAwsSpec, ...(value as Partial<BuildAgentPoolProviderSpecAwsEc2BuildAgentPoolProviderSpec>) };
  return {
    ...spec,
    $type: 'AwsEc2',
    provider: BuildAgentPoolProvider.AwsEc2,
    rootVolumeSizeGb: Number(spec.rootVolumeSizeGb ?? 50),
    securityGroupIds: Array.isArray(spec.securityGroupIds) ? spec.securityGroupIds : [],
    instanceProfileName: spec.instanceProfileName || null,
    awsCredentialSecretId: spec.awsCredentialSecretId || null,
    assumeRoleArn: spec.assumeRoleArn || null,
    keyPairName: spec.keyPairName || null,
    tags: spec.tags ?? null,
  };
}

function normalizeSelfManagedVmSpec(value: unknown): SelfManagedVmProviderSpec {
  const spec = {
    ...defaultSelfManagedVmSpec,
    ...(value as Partial<BuildAgentPoolProviderSpecSelfManagedVmBuildAgentPoolProviderSpec>),
  };
  return {
    ...spec,
    $type: 'SelfManagedVm',
    provider: BuildAgentPoolProvider.SelfManagedVm,
    endpoint: spec.endpoint ?? '',
    maxWorkers: Number(spec.maxWorkers ?? 1),
    registrationSecretId: spec.registrationSecretId || null,
    labels: Array.isArray(spec.labels) ? spec.labels : [],
  };
}

function getProvider(value: unknown): BuildAgentPoolProvider {
  const spec = value as { provider?: BuildAgentPoolProvider; $type?: string } | null | undefined;
  if (spec?.provider === BuildAgentPoolProvider.SelfManagedVm || spec?.$type === 'SelfManagedVm') {
    return BuildAgentPoolProvider.SelfManagedVm;
  }
  return BuildAgentPoolProvider.AwsEc2;
}

const architectureOptions = [
  { value: CpuArchitecture.Amd64, label: 'amd64' },
  { value: CpuArchitecture.Arm64, label: 'arm64' },
];

function splitCsv(value: string) {
  return value
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);
}
