import {
  BuildAgentPoolInput,
  BuildAgentPoolConnectionMode,
  BuildAgentPoolProvider,
  BuildAgentPoolProviderSpecAwsEc2BuildAgentPoolProviderSpec,
  BuildAgentPoolProviderSpecSelfManagedVmBuildAgentPoolProviderSpec,
  BuildAgentPoolValidationStatus,
  BuildAgentPoolView,
  CpuArchitecture,
  EdgeAgentEnrollmentView,
  EdgeAgentStatusView,
  ResourceControlState,
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
import { Button } from '@/components/ui/button';
import { ActivitiesTab } from '@/features/activities';
import { ResourceHeaderTagsEditor, ResourceTagSelector } from '@/features/tags/components';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { Constants } from '@/lib/constants';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { MonacoEditor } from '@/lib/monaco';
import { hasCapability } from '@/lib/resource-capabilities';
import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { useQueryClient } from '@tanstack/react-query';
import { HubConnection } from '@microsoft/signalr';
import { CheckCheck, Clipboard, KeyRound, Loader2 } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
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
  connectionMode: BuildAgentPoolConnectionMode.InboundAgent,
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
      Indicator: ({ resource }) => <BuildPoolHeaderIndicator pool={resource as BuildAgentPoolView} />,
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
      const [pool, setPool] = useState<BuildPoolFormResource | undefined>(data?.data as BuildPoolFormResource | undefined);
      const lastDataRef = useRef<BuildAgentPoolView | undefined>(data?.data);

      useEffect(() => {
        if (data?.data && data.data !== lastDataRef.current) {
          lastDataRef.current = data.data;
          setPool(data.data as BuildPoolFormResource);
        }
      }, [data?.data]);

      const handleBuildAgentPoolInfoUpdated = useCallback(
        (nextPool: BuildAgentPoolView) => {
          if (nextPool.id !== id) return;

          setPool(nextPool as BuildPoolFormResource);
        },
        [id],
      );

      const setupEventListeners = useCallback(
        (hubConnection: HubConnection) => {
          hubConnection.on('BuildAgentPoolInfoUpdated', handleBuildAgentPoolInfoUpdated);
        },
        [handleBuildAgentPoolInfoUpdated],
      );

      const removeEventListeners = useCallback(
        (hubConnection: HubConnection) => {
          hubConnection.off('BuildAgentPoolInfoUpdated', handleBuildAgentPoolInfoUpdated);
        },
        [handleBuildAgentPoolInfoUpdated],
      );

      useSignalRGroup({
        groupName: `build-agent-pool:${id}`,
        setupEventListeners,
        removeEventListeners,
      });

      return { item: pool, isLoading };
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
  const [enrollment, setEnrollment] = useState<EdgeAgentEnrollmentView | undefined>();
  const createPool = useMutate('createBuildAgentPool');
  const updatePool = useMutate('updateBuildAgentPool');
  const createEdgeEnrollment = useMutate('createBuildAgentPoolEdgeEnrollment');

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
  const isEdgePool =
    provider === BuildAgentPoolProvider.SelfManagedVm &&
    vmSpec.connectionMode === BuildAgentPoolConnectionMode.EdgeAgent;
  const { data: edgeStatusData, isLoading: isEdgeStatusLoading } = useRead(
    'getBuildAgentPoolEdgeStatus',
    { id: id ?? '' },
    { enabled: mode === 'edit' && Boolean(id) && isEdgePool },
  );
  const edgeStatus = edgeStatusData?.data;
  const regenerateEnrollment = useCallback(async () => {
    if (!id) return;
    const result = await createEdgeEnrollment.mutateAsync({ id });
    setEnrollment(result.data);
    await queryClient.invalidateQueries({ queryKey: ['getBuildAgentPoolEdgeStatus'] });
  }, [createEdgeEnrollment, id, queryClient]);
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
                id: 'provider-type',
                key: 'providerSpec',
                dirtyKey: 'providerSpec.provider',
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
            ? selfManagedVmFields(
                vmSpec,
                setVmSpec,
                mode,
                resource?.id,
                enrollment,
                edgeStatus,
                isEdgeStatusLoading,
                createEdgeEnrollment.isPending,
                regenerateEnrollment,
                disabled,
              )
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
    [
      awsSpec,
      createEdgeEnrollment.isPending,
      disabled,
      edgeStatus,
      enrollment,
      isEdgeStatusLoading,
      mode,
      provider,
      regenerateEnrollment,
      resource?.id,
      setAwsSpec,
      setProviderSpec,
      setVmSpec,
      vmSpec,
    ],
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

function BuildPoolHeaderIndicator({ pool }: { pool: BuildAgentPoolView }) {
  const title = [validationLabel(pool.lastValidationStatus), pool.lastValidationMessage].filter(Boolean).join(' - ');

  return (
    <span className="inline-flex items-center" title={title}>
      <StateIndicator
        value={pool.lastValidationStatus}
        kind="buildAgentPoolValidation"
        isProcessing={pool.controlState === ResourceControlState.Processing}
      />
    </span>
  );
}

function validationLabel(status: BuildAgentPoolValidationStatus) {
  switch (status) {
    case BuildAgentPoolValidationStatus.NotTested:
      return 'Not tested';
    case BuildAgentPoolValidationStatus.Ready:
      return 'Ready';
    case BuildAgentPoolValidationStatus.Invalid:
      return 'Invalid';
    case BuildAgentPoolValidationStatus.Degraded:
      return 'Degraded';
    default:
      return String(status);
  }
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
        id: 'aws-region',
        key: 'providerSpec',
        dirtyKey: 'providerSpec.region',
        label: 'Region',
        required: true,
        description: 'AWS region where builder instances are launched.',
        render: () => <FieldInput value={awsSpec.region} disabled={disabled} onChange={(region) => setAwsSpec({ region })} />,
      }),
      defineField({
        id: 'aws-instance-type',
        key: 'providerSpec',
        dirtyKey: 'providerSpec.instanceType',
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
        id: 'aws-architecture',
        key: 'providerSpec',
        dirtyKey: 'providerSpec.architecture',
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
        id: 'aws-ami-id',
        key: 'providerSpec',
        dirtyKey: 'providerSpec.amiId',
        label: 'AMI ID',
        required: true,
        description: 'AMI that boots the Citadel builder runtime.',
        render: () => <FieldInput value={awsSpec.amiId} disabled={disabled} onChange={(amiId) => setAwsSpec({ amiId })} />,
      }),
      defineField({
        id: 'aws-root-volume-size-gb',
        key: 'providerSpec',
        dirtyKey: 'providerSpec.rootVolumeSizeGb',
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
        id: 'aws-subnet-id',
        key: 'providerSpec',
        dirtyKey: 'providerSpec.subnetId',
        label: 'Subnet ID',
        required: true,
        description: 'Subnet where builders are launched.',
        render: () => <FieldInput value={awsSpec.subnetId} disabled={disabled} onChange={(subnetId) => setAwsSpec({ subnetId })} />,
      }),
      defineField({
        id: 'aws-security-group-ids',
        key: 'providerSpec',
        dirtyKey: 'providerSpec.securityGroupIds',
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
            id: 'aws-instance-profile',
            key: 'providerSpec',
            dirtyKey: 'providerSpec.instanceProfileName',
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
            id: 'aws-assume-role-arn',
            key: 'providerSpec',
            dirtyKey: 'providerSpec.assumeRoleArn',
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
        id: 'aws-assign-public-ip',
        key: 'providerSpec',
        dirtyKey: 'providerSpec.assignPublicIp',
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
        id: 'aws-key-pair',
        key: 'providerSpec',
        dirtyKey: 'providerSpec.keyPairName',
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
  mode: 'add' | 'edit',
  poolId?: string,
  enrollment?: EdgeAgentEnrollmentView,
  edgeStatus?: EdgeAgentStatusView,
  isEdgeStatusLoading?: boolean,
  isEnrollmentPending?: boolean,
  onRegenerateEnrollment?: () => void,
  disabled?: boolean,
) {
  const connectionMode = vmSpec.connectionMode ?? BuildAgentPoolConnectionMode.InboundAgent;
  const isEdge = connectionMode === BuildAgentPoolConnectionMode.EdgeAgent;

  return defineGroupField<BuildPoolInput>({
    id: 'self-managed-vm',
    label: 'Self-managed VM Settings',
    description: 'Settings for static builder hosts that are provisioned outside Citadel.',
    items: [
      defineField({
        id: 'self-managed-connection-mode',
        key: 'providerSpec',
        dirtyKey: 'providerSpec.connectionMode',
        label: 'Connection mode',
        required: true,
        description: 'Inbound Agent requires Core to reach the builder. Edge Agent uses the outbound Edge Agent channel.',
        render: () => (
          <FieldSelect
            value={connectionMode}
            disabled={disabled}
            onChange={(next) =>
              setVmSpec({
                connectionMode: next as BuildAgentPoolConnectionMode,
                endpoint: next === BuildAgentPoolConnectionMode.EdgeAgent ? null : (vmSpec.endpoint ?? ''),
              })
            }
            options={[
              { value: BuildAgentPoolConnectionMode.InboundAgent, label: 'Inbound Agent endpoint' },
              { value: BuildAgentPoolConnectionMode.EdgeAgent, label: 'Edge Agent' },
            ]}
          />
        ),
      }),
      ...(isEdge
        ? [
            defineField<BuildPoolInput>({
              id: 'self-managed-edge-enrollment',
              key: 'providerSpec',
              dirtyKey: 'providerSpec.__edgeEnrollment',
              label: 'Edge Agent enrollment',
              description:
                mode === 'add'
                  ? 'Save the build pool first, then generate the Edge Agent command from this pool.'
                  : 'This pool uses its own outbound Edge Agent connection. Generate or rotate the agent command from the saved pool.',
              render: () => (
                <EdgeBuildPoolEnrollmentPanel
                  poolId={poolId}
                  enrollment={enrollment}
                  edgeStatus={edgeStatus}
                  isEdgeStatusLoading={isEdgeStatusLoading}
                  isPending={isEnrollmentPending}
                  onRegenerate={onRegenerateEnrollment}
                  disabled={disabled}
                  mode={mode}
                />
              ),
            }),
          ]
        : [
            defineField<BuildPoolInput>({
              id: 'self-managed-endpoint',
              key: 'providerSpec',
              dirtyKey: 'providerSpec.endpoint',
              label: 'Endpoint',
              required: true,
              description: 'Stable URL used by Citadel Core to reach the inbound build agent.',
              render: () => (
                <FieldInput
                  value={vmSpec.endpoint ?? ''}
                  disabled={disabled}
                  onChange={(endpoint) => setVmSpec({ endpoint })}
                  placeholder="https://builder-01.example.com:9000"
                />
              ),
            }),
          ]),
      defineField({
        id: 'self-managed-architecture',
        key: 'providerSpec',
        dirtyKey: 'providerSpec.architecture',
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
        id: 'self-managed-max-workers',
        key: 'providerSpec',
        dirtyKey: 'providerSpec.maxWorkers',
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
            id: 'self-managed-registration-secret-id',
            key: 'providerSpec',
            dirtyKey: 'providerSpec.registrationSecretId',
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
            id: 'self-managed-labels',
            key: 'providerSpec',
            dirtyKey: 'providerSpec.labels',
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

function EdgeBuildPoolEnrollmentPanel({
  poolId,
  enrollment,
  edgeStatus,
  isEdgeStatusLoading,
  isPending,
  onRegenerate,
  disabled,
  mode,
}: {
  poolId?: string;
  enrollment?: EdgeAgentEnrollmentView;
  edgeStatus?: EdgeAgentStatusView;
  isEdgeStatusLoading?: boolean;
  isPending?: boolean;
  onRegenerate?: () => void;
  disabled?: boolean;
  mode: 'add' | 'edit';
}) {
  const dockerCommand = enrollment?.instructions.dockerRunCommand ?? '';
  const isSaved = mode === 'edit' && Boolean(poolId);
  const connectionStatus = edgeStatus?.connectionStatus;
  const isRevoked = connectionStatus === 'Revoked';
  const hasBinding = connectionStatus ? connectionStatus !== 'PendingEnrollment' : false;

  if (!isSaved) {
    return (
      <div className="rounded-md border border-dashed border-border bg-muted/20 p-4 text-sm text-muted-foreground">
        Save this build pool to generate its Edge Agent enrollment command.
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-start gap-3">
          <KeyRound className="mt-0.5 size-4 text-primary" />
          <div className="grid gap-1 text-sm">
            <div className="font-medium">
              {edgeStatus?.connectionStatus ?? (isEdgeStatusLoading ? 'Loading status...' : 'Pending enrollment')}
            </div>
            <div className="text-xs text-muted-foreground">
              {enrollment
                ? `Token expires ${formatDate(enrollment.expiresAtUtc)}. It is shown once.`
                : isRevoked
                  ? 'This build pool Edge Agent binding was revoked. Create a new build pool to enroll a fresh agent.'
                : edgeStatus?.enrollmentExpiresAtUtc
                  ? `Active token expires ${formatDate(edgeStatus.enrollmentExpiresAtUtc)}. Generate a new token to show a fresh value.`
                  : hasBinding
                    ? `Target address: edge-build-pool://${poolId}`
                    : 'Generate a one-time token and run the Docker command on the builder host.'}
            </div>
          </div>
        </div>

        <Button
          type="button"
          className="w-fit"
          variant={enrollment ? 'outline' : 'default'}
          disabled={disabled || isPending || isEdgeStatusLoading || isRevoked}
          onClick={onRegenerate}>
          {(isPending || isEdgeStatusLoading) && <Loader2 className="size-4 animate-spin" />}
          {isRevoked ? 'Binding Revoked' : hasBinding ? 'Rotate Enrollment Token' : enrollment ? 'Generate New Token' : 'Generate Enrollment Token'}
        </Button>
      </div>

      {dockerCommand && <SetupCommandEditor value={dockerCommand} filename="citadel-edge-build-agent.sh" />}
    </div>
  );
}

function SetupCommandEditor({ value, filename }: { value: string; filename: string }) {
  const [copied, copy] = useCopyToClipboard(3000);
  const isCopied = copied === value;

  return (
    <div className="relative max-w-full">
      <Button
        type="button"
        size="icon-xs"
        variant="outline"
        className="absolute right-5 top-4 z-10 bg-background/80"
        onClick={() => value && copy(value)}
        disabled={!value}>
        {isCopied ? <CheckCheck className="size-3 text-green-500" /> : <Clipboard className="size-3" />}
        <span className="sr-only">Copy docker command</span>
      </Button>
      <MonacoEditor
        value={value}
        language="shell"
        filename={filename}
        readOnly
        minHeight={180}
        className="mx-0 my-0"
        fontSize={12}
      />
    </div>
  );
}

function formatDate(value: unknown) {
  if (!value) return '-';
  return new Date(value as string).toLocaleString();
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
    endpoint: spec.connectionMode === BuildAgentPoolConnectionMode.EdgeAgent ? null : (spec.endpoint ?? ''),
    maxWorkers: Number(spec.maxWorkers ?? 1),
    registrationSecretId: spec.registrationSecretId || null,
    labels: Array.isArray(spec.labels) ? spec.labels : [],
    connectionMode: spec.connectionMode ?? BuildAgentPoolConnectionMode.InboundAgent,
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
