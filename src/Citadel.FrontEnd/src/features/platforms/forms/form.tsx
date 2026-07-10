import { PlatformConnectorType, PlatformType, PlatformView } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { defineField, defineGroupField, defineSection, FieldInput, FormShell } from '@/components/custom/form-builder';
import { Label } from '@/components/ui/label';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { ResourceTagSelector } from '@/features/tags/components';
import { Constants } from '@/lib/constants';
import { DockerIcon } from '@/lib/icons';
import { cn } from '@/lib/utils';
import { PlugZap, ShieldCheck } from 'lucide-react';
import { useMemo, useState } from 'react';
import { PlatformEnrollmentActions } from './actions';
import {
  createDefaultPlatformInput,
  platformToFormInput,
  PlatformFormInput,
  usePlatformForm,
} from './hooks/usePlatformForm';

const platformOptions = {
  [PlatformType.Docker]: {
    label: 'Docker Standalone',
    description: 'Connect a Docker host through an agent.',
    icon: DockerIcon,
    disabled: false,
  },
  [PlatformType.DockerSwarm]: {
    label: 'Docker Swarm',
    description: 'Manage a cluster of Docker daemons.',
    icon: DockerIcon,
    disabled: true,
  },
  [PlatformType.Kubernetes]: {
    label: 'Kubernetes',
    description: 'Manage a Kubernetes cluster.',
    icon: DockerIcon,
    disabled: true,
  },
} as const;

const connectorOptions = {
  [PlatformConnectorType.Agent]: {
    label: 'Inbound Agent',
    description: 'Citadel Core connects to the agent address.',
    icon: PlugZap,
  },
  [PlatformConnectorType.EdgeAgent]: {
    label: 'Edge Agent',
    description: 'The agent opens an outbound connection to Citadel Core.',
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
  const original = useMemo(
    () => (mode === 'edit' && resource ? platformToFormInput(resource) : createDefaultPlatformInput()),
    [mode, resource],
  );
  const { createdPlatform, enrollment, isPending, validationErrors, save, regenerateEnrollment } = usePlatformForm(
    mode,
    resource,
  );

  const connectorType = update.connectorType ?? original.connectorType ?? PlatformConnectorType.Agent;
  const isEdge = connectorType === PlatformConnectorType.EdgeAgent;
  const isEdit = mode === 'edit';
  const formDisabled = disabled || Boolean(createdPlatform);

  const schema = useMemo(
    () => ({
      '': defineSection<PlatformFormInput>({
        title: '',
        items: [
          defineGroupField<PlatformFormInput>({
            id: 'details',
            label: 'Details',
            items: [
              defineField({
                key: 'type',
                label: 'Platform Type',
                required: true,
                description: 'Choose the platform runtime to connect.',
                render: (value, set) => (
                  <PlatformTypeSelector
                    value={value ?? PlatformType.Docker}
                    onChange={(type) => set({ type })}
                    disabled={formDisabled || isEdit}
                  />
                ),
              }),
              defineField({
                key: 'connectorType',
                label: 'Connection',
                required: true,
                description: 'Choose how Citadel Core reaches this platform.',
                render: (value, set) => (
                  <ConnectorTypeSelector
                    value={value ?? PlatformConnectorType.Agent}
                    disabled={formDisabled || isEdit}
                    onChange={(next) =>
                      set((prev) => ({
                        ...prev,
                        connectorType: next,
                        address: next === PlatformConnectorType.EdgeAgent ? null : prev.address,
                      }))
                    }
                  />
                ),
              }),
              ...(isEdit
                ? []
                : [
                    defineField({
                      key: 'name',
                      label: 'Name',
                      required: true,
                      description: 'Provide a unique name for this platform.',
                      validate: (value) =>
                        !new RegExp(Constants.validNameIdentifier).test(value) ? 'Invalid name format' : null,
                      render: (value, set) => (
                        <FieldInput
                          value={value ?? ''}
                          onChange={(name) => set({ name })}
                          placeholder="docker-prod-01"
                          disabled={formDisabled}
                        />
                      ),
                    }),
                  ]),
              ...(!isEdge
                ? [
                    defineField<PlatformFormInput, 'address'>({
                      key: 'address',
                      label: 'Agent Address',
                      required: true,
                      description: 'HTTP or HTTPS address where Citadel Core can reach the inbound agent.',
                      validate: (value) => (!value ? 'Required' : null),
                      render: (value, set) => (
                        <FieldInput
                          value={value ?? ''}
                          onChange={(address) => set({ address })}
                          placeholder="https://192.168.1.25:9000"
                          disabled={formDisabled}
                        />
                      ),
                    }),
                  ]
                : []),
              ...(isEdit
                ? []
                : [
                    defineField({
                      key: 'tagIds',
                      label: 'Tags',
                      required: false,
                      description: 'Optional tags for filtering and grouping this platform.',
                      render: (value, set) => (
                        <ResourceTagSelector
                          value={value ?? []}
                          onChange={(tagIds) => set({ tagIds })}
                          disabled={formDisabled}
                        />
                      ),
                    }),
                  ]),
            ],
          }),
        ],
      }),
    }),
    [formDisabled, isEdge, isEdit],
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

      {(createdPlatform || (isEdit && resource?.connectorType === PlatformConnectorType.EdgeAgent)) && (
        <PlatformEnrollmentActions
          platformName={(createdPlatform ?? resource)?.name}
          enrollment={enrollment}
          isPending={isPending}
          onRegenerate={regenerateEnrollment}
        />
      )}
    </div>
  );
};

const PlatformTypeSelector = ({
  value,
  onChange,
  disabled,
}: {
  value: PlatformType;
  onChange: (value: PlatformType) => void;
  disabled?: boolean;
}) => (
  <RadioGroup
    value={value}
    onValueChange={(next) => onChange(next as PlatformType)}
    className="grid gap-3 md:grid-cols-3">
    {Object.entries(platformOptions).map(([optionValue, option]) => {
      const Icon = option.icon;
      return (
        <SelectorOption
          key={optionValue}
          value={optionValue}
          label={option.label}
          description={option.description}
          disabled={disabled || option.disabled}>
          <Icon className="size-4" />
        </SelectorOption>
      );
    })}
  </RadioGroup>
);

const ConnectorTypeSelector = ({
  value,
  onChange,
  disabled,
}: {
  value: PlatformConnectorType;
  onChange: (value: PlatformConnectorType) => void;
  disabled?: boolean;
}) => (
  <RadioGroup
    value={value}
    onValueChange={(next) => onChange(next as PlatformConnectorType)}
    className="grid gap-3 md:grid-cols-2">
    {Object.entries(connectorOptions).map(([optionValue, option]) => {
      const Icon = option.icon;
      return (
        <SelectorOption
          key={optionValue}
          value={optionValue}
          label={option.label}
          description={option.description}
          disabled={disabled}>
          <Icon className="size-4" />
        </SelectorOption>
      );
    })}
  </RadioGroup>
);

const SelectorOption = ({
  value,
  label,
  description,
  disabled,
  children,
}: {
  value: string;
  label: string;
  description: string;
  disabled?: boolean;
  children: React.ReactNode;
}) => (
  <Label
    className={cn(
      'flex min-h-24 cursor-pointer items-start gap-3 rounded-lg border p-4 hover:bg-accent/50',
      'has-[[data-state=checked]]:border-primary/50 has-[[data-state=checked]]:bg-primary/10',
      disabled && 'cursor-not-allowed opacity-60 hover:bg-transparent',
    )}>
    <RadioGroupItem
      value={value}
      disabled={disabled}
      className="mt-0.5 shadow-none data-[state=checked]:border-primary/50 data-[state=checked]:bg-primary *:data-[slot=radio-group-indicator]:[&>svg]:fill-white *:data-[slot=radio-group-indicator]:[&>svg]:stroke-white"
    />
    <div className="grid gap-1 font-normal">
      <div className="flex items-center gap-2 font-medium">
        {children}
        {label}
      </div>
      <div className="text-sm leading-snug text-muted-foreground">{description}</div>
    </div>
  </Label>
);
