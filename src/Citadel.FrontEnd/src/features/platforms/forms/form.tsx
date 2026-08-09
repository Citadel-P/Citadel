import {
  AgentSetupView,
  EdgeAgentEnrollmentView,
  EdgeAgentStatusView,
  PlatformConnectorType,
  PlatformType,
  PlatformView,
} from '@/api/generated/api.types';
import { ActionWithDialog } from '@/components/custom/action-with-dialog';
import { AlertMessage } from '@/components/custom/alert-message';
import {
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldSwitch,
  FieldTextArea,
  FormShell,
} from '@/components/custom/form-builder';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { ResourceTagSelector } from '@/features/tags/components';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { useMutate, useRead } from '@/lib/hooks';
import { MonacoEditor } from '@/lib/monaco';
import { useQueryClient } from '@tanstack/react-query';
import { CheckCheck, Clipboard, KeyRound, Loader2, PlugZap, RefreshCw, Server, ShieldCheck } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import { toast } from 'sonner';
import {
  createDefaultPlatformInput,
  platformToFormInput,
  PlatformFormInput,
  usePlatformForm,
} from './hooks/usePlatformForm';

const platformTypeInfo = {
  [PlatformType.Docker]: {
    label: 'Docker Standalone',
    description: 'Manage one Docker Engine and its local resources.',
  },
  [PlatformType.DockerSwarm]: {
    label: 'Docker Swarm',
    description: 'Manage an existing Docker Swarm through a manager node.',
  },
} as const;

const connectorInfo = {
  [PlatformConnectorType.Local]: {
    label: 'Local',
    description: 'Connect through the Docker socket mounted on Citadel Core.',
    icon: Server,
  },
  [PlatformConnectorType.Agent]: {
    label: 'Agent',
    description: 'Connect to a Citadel Agent exposed on the target host.',
    icon: PlugZap,
  },
  [PlatformConnectorType.EdgeAgent]: {
    label: 'Edge Agent',
    description: 'Connect through a Citadel Agent that initiates a secure outbound connection.',
    icon: ShieldCheck,
  },
} as const;

export const PlatformForm = ({
  mode,
  resource,
  disabled,
}: {
  mode: 'add' | 'edit';
  resource?: PlatformView;
  disabled?: boolean;
}) => {
  const [update, setUpdate] = useState<Partial<PlatformFormInput>>({});
  const queryClient = useQueryClient();
  const { data: platformsData } = useRead('listPlatforms');
  const rotateAgentHubKeyMutation = useMutate('rotateAgentHubKey');
  const { mutateAsync: rotateAgentHubKeyAsync, isPending: isRotatingAgentHubKey } = rotateAgentHubKeyMutation;
  const localPlatformExists = useMemo(
    () =>
      (platformsData?.data.platforms ?? []).some(
        (platform) => platform.connectorType === PlatformConnectorType.Local && platform.id !== resource?.id,
      ),
    [platformsData?.data.platforms, resource?.id],
  );
  const isLocalConnectorDisabled = mode === 'add' && localPlatformExists;
  const defaultConnectorType = isLocalConnectorDisabled ? PlatformConnectorType.Agent : PlatformConnectorType.Local;
  const original = useMemo(
    () =>
      mode === 'edit' && resource ? platformToFormInput(resource) : createDefaultPlatformInput(defaultConnectorType),
    [mode, resource, defaultConnectorType],
  );
  const { createdPlatform, enrollment, isPending, validationErrors, save, regenerateEnrollment } = usePlatformForm(
    mode,
    resource,
  );

  const enrollmentPlatform = createdPlatform ?? resource;
  const { data: edgeStatusData, isLoading: isEdgeStatusLoading } = useRead(
    'getEdgeAgentStatus',
    { id: enrollmentPlatform?.id ?? '' },
    {
      enabled: Boolean(enrollmentPlatform?.id) && enrollmentPlatform?.connectorType === PlatformConnectorType.EdgeAgent,
    },
  );
  const edgeStatus = edgeStatusData?.data;
  const connectorType =
    createdPlatform?.connectorType ?? update.connectorType ?? original.connectorType ?? PlatformConnectorType.Local;
  const isAgent = connectorType === PlatformConnectorType.Agent;
  const isEdge = connectorType === PlatformConnectorType.EdgeAgent;
  const platformType = update.type ?? original.type ?? PlatformType.Docker;
  const isSwarm = platformType === PlatformType.DockerSwarm;
  const { data: agentSetupData, isLoading: isAgentSetupLoading } = useRead('getAgentSetup', undefined, {
    enabled: isAgent,
  });
  const agentSetup: AgentSetupView | undefined = agentSetupData?.data;
  const isEdit = mode === 'edit';
  const formDisabled = disabled || Boolean(createdPlatform);
  const canGenerateEnrollment = Boolean(enrollmentPlatform?.id);
  const isEnrolled = edgeStatus ? edgeStatus.connectionStatus !== 'PendingEnrollment' : false;
  const dockerCommand = enrollment?.instructions.dockerRunCommand ?? '';
  const rotateAgentHubKey = useCallback(async () => {
    await rotateAgentHubKeyAsync({});
    await queryClient.invalidateQueries({ queryKey: ['getAgentSetup'] });
    toast.success('Agent public key rotated', {
      description: 'Restart every regular Agent with the new public key before using those platforms.',
    });
  }, [queryClient, rotateAgentHubKeyAsync]);

  const schema = useMemo(
    () => ({
      '': defineSection<PlatformFormInput>({
        title: '',
        items: [
          ...(mode === 'add'
            ? [
                defineGroupField<PlatformFormInput>({
                  id: 'details',
                  label: 'Details',
                  items: [
                    defineField({
                      key: 'name',
                      label: 'Name',
                      required: true,
                      description: 'Provide a unique name for this platform.',
                      validate: (v) => (!v ? 'Name is required' : null),
                      render: (val, set) => (
                        <FieldInput value={val} onChange={(v) => set({ name: v })} placeholder="e.g. platfom-01" />
                      ),
                    }),
                    defineField({
                      key: 'description',
                      label: 'Description',
                      required: false,
                      description: 'Optional description.',
                      render: (val, set) => <FieldTextArea value={val} onChange={(v) => set({ description: v })} />,
                    }),
                    defineField({
                      key: 'tagIds',
                      label: 'Tags',
                      required: false,
                      description: 'Optional tags for filtering and grouping this platform.',
                      render: (val, set) => (
                        <ResourceTagSelector value={val} disabled={disabled} onChange={(tagIds) => set({ tagIds })} />
                      ),
                    }),
                  ],
                }),
              ]
            : []),
          defineGroupField<PlatformFormInput>({
            id: 'runtime',
            label: 'Runtime',
            items: [
              defineField({
                key: 'type',
                label: 'Platform Type',
                required: true,
                description:
                  mode === 'edit'
                    ? 'Platform type is fixed after creation.'
                    : 'Choose Docker Standalone for one daemon or Docker Swarm for an existing manager node.',
                render: (value, set) => (
                  <PlatformTypeSelector
                    value={value ?? PlatformType.Docker}
                    onChange={(type) => set({ type })}
                    disabled={formDisabled || isEdit}
                  />
                ),
              }),
            ],
          }),
          ...(isSwarm
            ? [
                defineGroupField<PlatformFormInput>({
                  id: 'maintenance',
                  label: 'Maintenance',
                  items: [
                    defineField({
                      key: 'pruneHistoricalSwarmTaskContainers',
                      label: 'Prune historical task containers',
                      description:
                        'Delete terminal Swarm task containers retained on the connected manager. Running tasks and standalone or Compose containers are never removed.',
                      render: (value, set) => (
                        <FieldSwitch
                          id="prune-historical-swarm-task-containers"
                          checked={value !== false}
                          onChange={(pruneHistoricalSwarmTaskContainers) =>
                            set({ pruneHistoricalSwarmTaskContainers })
                          }
                          disabled={formDisabled}
                        />
                      ),
                    }),
                  ],
                }),
              ]
            : []),
          defineGroupField<PlatformFormInput>({
            id: 'connector',
            label: 'Connector',
            items: [
              defineField({
                key: 'connectorType',
                label: 'Connector',
                required: true,
                description: 'Choose how Citadel Core reaches this platform.',
                validate: (value) =>
                  isLocalConnectorDisabled && value === PlatformConnectorType.Local
                    ? 'A Local platform is already configured.'
                    : null,
                render: (value, set) => (
                  <ConnectorTypeSelector
                    value={value ?? defaultConnectorType}
                    disabled={formDisabled || isEdit}
                    localDisabled={isLocalConnectorDisabled}
                    onChange={(next) =>
                      set((prev) => ({
                        ...prev,
                        connectorType: next,
                        address: next === PlatformConnectorType.Agent ? (prev.address ?? '') : null,
                      }))
                    }
                  />
                ),
              }),
              ...(isAgent
                ? [
                    defineField<PlatformFormInput, 'address'>({
                      key: 'address',
                      label: 'Agent Address',
                      required: true,
                      description: agentSetup?.requiresTls
                        ? 'HTTPS address where Citadel Core can reach the inbound agent.'
                        : 'HTTP or HTTPS address where Citadel Core can reach the inbound agent.',
                      validate: (value) => {
                        if (!value) return 'Required';
                        if (agentSetup?.requiresTls && !value.toLowerCase().startsWith('https://')) {
                          return 'HTTPS is required by the Core Agent transport policy.';
                        }
                        return null;
                      },
                      render: (value, set) => (
                        <FieldInput
                          value={value ?? ''}
                          onChange={(address) => set({ address })}
                          placeholder={
                            agentSetup?.requiresTls ? 'https://192.168.1.25:9000' : 'http://192.168.1.25:9000'
                          }
                          disabled={formDisabled}
                        />
                      ),
                    }),
                  ]
                : []),
            ],
          }),
          ...(isAgent
            ? [
                defineGroupField<PlatformFormInput>({
                  id: 'agent-setup',
                  label: 'Agent Setup',
                  title: 'Agent Setup',
                  description:
                    'Run this agent on the Docker host before saving the platform. Citadel Core keeps the private key and the agent receives only the public key.',
                  items: [
                    defineField<PlatformFormInput, 'agentHubPublicKey'>({
                      key: 'agentHubPublicKey',
                      label: 'Hub Public Key',
                      description: 'Public key used by the Agent to verify requests signed by Citadel Core.',
                      disabled: false,
                      ignoreFormDisabled: true,
                      hideValidationMessage: true,
                      render: () => (
                        <AgentPublicKeyField
                          value={agentSetup?.hubPublicKey ?? ''}
                          placeholder={isAgentSetupLoading ? 'Loading setup details...' : ''}
                          disabled={disabled || isRotatingAgentHubKey}
                          onRotate={rotateAgentHubKey}
                        />
                      ),
                    }),
                    defineField<PlatformFormInput, 'agentDockerCommand'>({
                      key: 'agentDockerCommand',
                      label: 'Docker Command',
                      description: agentSetup?.requiresTls
                        ? 'Replace the TLS host path in this command template, then run it on the Docker host.'
                        : 'Run this command on the Docker host, then enter the host address above.',
                      disabled: false,
                      ignoreFormDisabled: true,
                      hideValidationMessage: true,
                      render: () => (
                        <SetupCommandEditor value={agentSetup?.dockerRunCommand ?? ''} filename="citadel-agent.sh" />
                      ),
                    }),
                  ],
                }),
              ]
            : []),
          ...(isEdge
            ? [
                defineGroupField<PlatformFormInput>({
                  id: 'enrollment',
                  label: 'Enrollment',
                  title: 'Enrollment',
                  description: canGenerateEnrollment
                    ? getEnrollmentDescription(isEnrolled)
                    : 'Save this Edge Agent platform before generating enrollment instructions.',
                  items: [
                    defineField<PlatformFormInput, 'edgeEnrollmentAction'>({
                      key: 'edgeEnrollmentAction',
                      label: 'Token',
                      disabled,
                      ignoreFormDisabled: true,
                      hideValidationMessage: true,
                      render: () =>
                        canGenerateEnrollment ? (
                          <EnrollmentTokenField
                            enrollment={enrollment}
                            edgeStatus={edgeStatus}
                            isEdgeStatusLoading={isEdgeStatusLoading}
                            isEnrolled={isEnrolled}
                            isPending={isPending}
                            onRegenerate={regenerateEnrollment}
                          />
                        ) : (
                          <EdgeEnrollmentPending />
                        ),
                    }),
                    ...(enrollment
                      ? [
                          defineField<PlatformFormInput, 'edgeDockerCommand'>({
                            key: 'edgeDockerCommand',
                            label: 'Docker Command',
                            description: 'Run this command on the host that should connect back to Citadel.',
                            disabled: false,
                            ignoreFormDisabled: true,
                            hideValidationMessage: true,
                            render: () => <SetupCommandEditor value={dockerCommand} filename="citadel-edge-agent.sh" />,
                          }),
                        ]
                      : []),
                  ],
                }),
              ]
            : []),
        ],
      }),
    }),
    [
      agentSetup?.dockerRunCommand,
      agentSetup?.hubPublicKey,
      agentSetup?.requiresTls,
      canGenerateEnrollment,
      defaultConnectorType,
      disabled,
      dockerCommand,
      edgeStatus,
      enrollment,
      formDisabled,
      isAgent,
      isAgentSetupLoading,
      isEdge,
      isEdgeStatusLoading,
      isEnrolled,
      isEdit,
      isLocalConnectorDisabled,
      isPending,
      mode,
      regenerateEnrollment,
      rotateAgentHubKey,
      isRotatingAgentHubKey,
      isSwarm,
    ],
  );

  return (
    <div className="flex flex-col gap-6">
      {validationErrors && <AlertMessage type="warning">{validationErrors}</AlertMessage>}

      <FormShell
        mode={mode}
        schema={schema}
        original={original}
        update={update}
        setUpdate={setUpdate}
        onSave={save}
        pending={isPending}
        disabled={formDisabled}
        draftKey={isEdit ? `platform:${resource?.id}:config` : 'platform:new'}
        draftVersion={1}
      />
    </div>
  );
};

const EdgeEnrollmentPending = () => (
  <div className="rounded-md border border-dashed border-border bg-muted/20 p-4 text-sm text-muted-foreground">
    Save this Edge Agent platform to generate enrollment instructions.
  </div>
);

const EnrollmentTokenField = ({
  enrollment,
  edgeStatus,
  isEdgeStatusLoading,
  isEnrolled,
  isPending,
  onRegenerate,
}: {
  enrollment?: EdgeAgentEnrollmentView;
  edgeStatus?: EdgeAgentStatusView;
  isEdgeStatusLoading?: boolean;
  isEnrolled: boolean;
  isPending?: boolean;
  onRegenerate: () => void;
}) => {
  const actionLabel = getEnrollmentActionLabel(Boolean(enrollment), isEnrolled);
  const detail = getEnrollmentStatusDetail(enrollment, edgeStatus, isEnrolled);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-start gap-3">
          <KeyRound className="mt-0.5 size-4 text-primary" />
          <div className="grid gap-1 text-sm">
            <div className="font-medium">
              {edgeStatus?.connectionStatus ?? (isEdgeStatusLoading ? 'Loading status...' : 'Pending enrollment')}
            </div>
            {detail && <div className="text-xs text-muted-foreground">{detail}</div>}
          </div>
        </div>

        <Button
          type="button"
          className="w-fit"
          variant={enrollment ? 'outline' : 'default'}
          onClick={onRegenerate}
          disabled={isPending || isEdgeStatusLoading}>
          {(isPending || isEdgeStatusLoading) && <Loader2 className="size-4 animate-spin" />}
          {actionLabel}
        </Button>
      </div>
    </div>
  );
};

const AgentPublicKeyField = ({
  value,
  placeholder,
  disabled,
  onRotate,
}: {
  value: string;
  placeholder?: string;
  disabled?: boolean;
  onRotate: () => Promise<void>;
}) => (
  <div className="flex max-w-full flex-col gap-3">
    <CopyableInput value={value} placeholder={placeholder} />
    <div className="flex flex-col gap-2 rounded-md border border-warning/40 bg-warning/10 p-3 text-xs text-muted-foreground">
      <div>
        Rotating this key changes how Citadel Core signs requests to regular Agents. Existing regular Agents must be
        restarted with the new public key before they can accept Core requests again.
      </div>
      <ActionWithDialog
        name="rotate"
        title="Rotate Key"
        icon={<RefreshCw className="size-4" />}
        iconPosition="left"
        variant="outline"
        disabled={disabled || !value}
        onClick={onRotate}
        targetClassName="w-fit max-w-none flex-none"
        additional={
          <p className="text-sm text-muted-foreground">
            After rotation, copy the new Docker command or public key and restart every regular Agent container. Edge
            Agents are not affected by this key.
          </p>
        }
      />
    </div>
  </div>
);

const CopyableInput = ({ value, placeholder }: { value: string; placeholder?: string }) => {
  const [copied, copy] = useCopyToClipboard(3000);
  const isCopied = copied === value;

  return (
    <div className="relative max-w-full">
      <FieldInput
        value={value}
        onChange={() => undefined}
        readOnly
        placeholder={placeholder}
        className="pr-10 font-mono text-xs"
      />
      <Button
        type="button"
        size="icon-xs"
        variant="outline"
        className="absolute left-92 top-1/2 -translate-y-1/2"
        onClick={() => value && copy(value)}
        disabled={!value}>
        {isCopied ? <CheckCheck className="size-3 text-green-500" /> : <Clipboard className="size-3" />}
        <span className="sr-only">Copy value</span>
      </Button>
    </div>
  );
};

const SetupCommandEditor = ({ value, filename }: { value: string; filename: string }) => {
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
};

const getEnrollmentDescription = (isEnrolled: boolean) =>
  isEnrolled
    ? 'This platform is enrolled. Rotate the token only when replacing or re-enrolling the Edge Agent.'
    : 'Generate a one-time token and run the generated Docker command on the target host.';

const getEnrollmentStatusDetail = (
  enrollment: EdgeAgentEnrollmentView | undefined,
  edgeStatus: EdgeAgentStatusView | undefined,
  isEnrolled: boolean,
) => {
  if (enrollment) {
    return `Token expires ${formatDate(enrollment.expiresAtUtc)}. It is shown once.`;
  }

  if (edgeStatus?.enrollmentExpiresAtUtc) {
    return `Active token expires ${formatDate(edgeStatus.enrollmentExpiresAtUtc)}. Generate a new token to show a fresh value.`;
  }

  return isEnrolled ? null : 'No token has been generated yet.';
};

const getEnrollmentActionLabel = (hasVisibleEnrollment: boolean, isEnrolled: boolean) => {
  if (isEnrolled) return 'Rotate Enrollment Token';
  if (hasVisibleEnrollment) return 'Generate New Token';

  return 'Generate Enrollment Token';
};

const formatDate = (value: unknown) => {
  if (!value) return '-';
  return new Date(value as string).toLocaleString();
};

const PlatformTypeSelector = ({
  value,
  onChange,
  disabled,
}: {
  value: PlatformType;
  onChange: (value: PlatformType) => void;
  disabled?: boolean;
}) => {
  const selected = platformTypeInfo[value as keyof typeof platformTypeInfo] ?? platformTypeInfo[PlatformType.Docker];

  return (
    <Select value={value} onValueChange={(next) => onChange(next as PlatformType)} disabled={disabled}>
      <SelectTrigger className="w-full max-w-100">
        <SelectValue>{selected.label}</SelectValue>
      </SelectTrigger>

      <SelectContent className="bg-background">
        {Object.entries(platformTypeInfo).map(([key, info]) => (
          <SelectItem key={key} value={key}>
            <div className="flex flex-col">
              <span className="font-medium">{info.label}</span>
              <span className="text-xs text-muted-foreground">{info.description}</span>
            </div>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
};

const ConnectorTypeSelector = ({
  value,
  onChange,
  disabled,
  localDisabled,
}: {
  value: PlatformConnectorType;
  onChange: (value: PlatformConnectorType) => void;
  disabled?: boolean;
  localDisabled?: boolean;
}) => {
  const selected = connectorInfo[value as keyof typeof connectorInfo] ?? connectorInfo[PlatformConnectorType.Local];
  const SelectedIcon = selected.icon;

  return (
    <Select
      value={value}
      onValueChange={(next) => {
        if (localDisabled && next === PlatformConnectorType.Local) return;
        onChange(next as PlatformConnectorType);
      }}
      disabled={disabled}>
      <SelectTrigger className="w-full max-w-100">
        <SelectValue>
          <div className="flex items-center gap-2">
            <SelectedIcon className="size-4" />
            <span>{selected.label}</span>
          </div>
        </SelectValue>
      </SelectTrigger>

      <SelectContent className="bg-background">
        {Object.entries(connectorInfo).map(([key, info]) => {
          const Icon = info.icon;
          const isLocal = key === PlatformConnectorType.Local;
          return (
            <SelectItem key={key} value={key} disabled={localDisabled && isLocal}>
              <div className="flex items-center gap-2">
                <Icon className="size-4" />
                <div className="flex flex-col">
                  <span className="font-medium">{info.label}</span>
                  <span className="text-xs text-muted-foreground">
                    {localDisabled && isLocal ? 'A Local platform is already configured.' : info.description}
                  </span>
                </div>
              </div>
            </SelectItem>
          );
        })}
      </SelectContent>
    </Select>
  );
};
