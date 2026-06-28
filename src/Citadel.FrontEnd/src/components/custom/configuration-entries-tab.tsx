import {
  ConfigurationEntriesView,
  ConfigurationEntryInput,
  ConfigurationEntryKind,
  ConfigurationEntryView,
  ConfigurationScope,
  CreateInternalSecretInput,
  SecretDefinitionView,
} from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useMutate, useRead } from '@/lib/hooks';
import { cn } from '@/lib/utils';
import { KeyRound, Plus, Save, Trash2, Variable } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';

const ENV_DELIVERY_MODE = 'EnvironmentVariable';

type EditableEntry = ConfigurationEntryInput & {
  clientId: string;
};

export const ConfigurationSummary = ({
  scope,
  resourceId,
  title = 'Runtime environment',
  description = 'Manage variables and secret bindings in the Environment tab. Secrets are resolved only during deploy.',
}: {
  scope: ConfigurationScope.Stack | ConfigurationScope.Deployment;
  resourceId: string;
  title?: string;
  description?: string;
}) => {
  const args = useMemo(() => ({ scope, resourceId }), [resourceId, scope]);
  const { data, isLoading } = useRead('getResourceConfigurationEntries', args);
  const effectiveEntries = data?.data.effectiveEntries ?? [];
  const variableCount = effectiveEntries.filter((entry) => entry.kind === ConfigurationEntryKind.Variable).length;
  const secretCount = effectiveEntries.filter((entry) => entry.kind === ConfigurationEntryKind.Secret).length;

  const openEnvironmentTab = () => {
    window.location.hash = 'environment';
  };

  return (
    <div className="rounded-sm border border-dashed bg-muted/20 px-4 py-3">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex flex-col gap-1">
          <div className="text-sm font-medium">{title}</div>
          <div className="text-xs text-muted-foreground">{description}</div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <span className="inline-flex items-center gap-1 rounded-sm border px-2 py-1 text-xs text-muted-foreground">
            <Variable className="size-3" />
            {isLoading ? '-' : variableCount} variables
          </span>
          <span className="inline-flex items-center gap-1 rounded-sm border border-dashed px-2 py-1 text-xs text-muted-foreground">
            <KeyRound className="size-3" />
            {isLoading ? '-' : secretCount} secrets
          </span>
          <Button type="button" variant="outline" size="sm" onClick={openEnvironmentTab}>
            Open Environment
          </Button>
        </div>
      </div>
    </div>
  );
};

export const ConfigurationEntriesTab = ({
  scope,
  resourceId,
  disabled,
}: {
  scope: ConfigurationScope.Stack | ConfigurationScope.Deployment;
  resourceId: string;
  disabled?: boolean;
}) => {
  const queryClient = useQueryClient();
  const args = useMemo(() => ({ scope, resourceId }), [resourceId, scope]);
  const { data, isLoading } = useRead('getResourceConfigurationEntries', args);
  const { data: secretsData } = useRead('listSecretDefinitions');
  const replace = useMutate('replaceResourceConfigurationEntries');
  const createSecret = useMutate('createInternalSecret');
  const [entries, setEntries] = useState<EditableEntry[]>([]);
  const [secretDialogOpen, setSecretDialogOpen] = useState(false);
  const [secretInput, setSecretInput] = useState<CreateInternalSecretInput>({ name: '', value: '' });

  useEffect(() => {
    if (!data?.data.entries) return;
    setEntries(data.data.entries.map(toEditableEntry));
  }, [data?.data.entries]);

  const hasChanges = useMemo(() => {
    const current = entries.map(toInput);
    const original = (data?.data.entries ?? []).map(toInputFromView);
    return JSON.stringify(current) !== JSON.stringify(original);
  }, [data?.data.entries, entries]);

  const secrets = secretsData?.data.secrets ?? [];

  const updateEntry = (clientId: string, patch: Partial<EditableEntry>) => {
    setEntries((prev) => prev.map((entry) => (entry.clientId === clientId ? { ...entry, ...patch } : entry)));
  };

  const removeEntry = (clientId: string) => {
    setEntries((prev) => prev.filter((entry) => entry.clientId !== clientId));
  };

  const addVariable = () => {
    setEntries((prev) => [
      ...prev,
      {
        clientId: crypto.randomUUID(),
        name: '',
        kind: ConfigurationEntryKind.Variable,
        value: '',
        secretId: null,
        secretDeliveryMode: null,
        targetPath: null,
      },
    ]);
  };

  const addSecretBinding = (secret?: SecretDefinitionView) => {
    setEntries((prev) => [
      ...prev,
      {
        clientId: crypto.randomUUID(),
        name: secret?.name ?? '',
        kind: ConfigurationEntryKind.Secret,
        value: null,
        secretId: secret?.id ?? null,
        secretDeliveryMode: ENV_DELIVERY_MODE,
        targetPath: null,
      },
    ]);
  };

  const overrideEntry = (entry: ConfigurationEntryView) => {
    if (entries.some((local) => local.name === entry.name)) {
      toast.info(`${entry.name} already has a resource override`);
      return;
    }

    setEntries((prev) => [
      ...prev,
      {
        clientId: crypto.randomUUID(),
        name: entry.name,
        kind: entry.kind,
        value: entry.kind === ConfigurationEntryKind.Variable ? (entry.value ?? '') : null,
        secretId: entry.kind === ConfigurationEntryKind.Secret ? entry.secretId : null,
        secretDeliveryMode:
          entry.kind === ConfigurationEntryKind.Secret ? (entry.secretDeliveryMode ?? ENV_DELIVERY_MODE) : null,
        targetPath: entry.kind === ConfigurationEntryKind.Secret ? (entry.targetPath ?? null) : null,
      },
    ]);
  };

  const removeOverride = (name: string) => {
    setEntries((prev) => prev.filter((entry) => entry.name !== name));
  };

  const save = async () => {
    try {
      await replace.mutateAsync({
        scope,
        resourceId,
        data: { entries: entries.map(toInput) },
      } as any);
      await queryClient.invalidateQueries({ queryKey: ['getResourceConfigurationEntries', args] });
      toast.success('Variables and secrets saved');
    } catch {
      toast.error(replace.validationErrors ?? 'Failed to save variables and secrets');
    }
  };

  const createInternalSecret = async () => {
    try {
      const result = await createSecret.mutateAsync({ data: secretInput } as any);
      addSecretBinding(result.data);
      setSecretInput({ name: '', value: '' });
      setSecretDialogOpen(false);
      await queryClient.invalidateQueries({ queryKey: ['listSecretDefinitions', {}] });
      toast.success('Secret created');
    } catch {
      toast.error(createSecret.validationErrors ?? 'Failed to create secret');
    }
  };

  return (
    <div className="flex w-full flex-col gap-4">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <div className="text-sm text-muted-foreground">
          Define environment variables and secret bindings available during apply. Resource values override global values.
        </div>
        <div className="flex flex-wrap gap-2">
          <Button type="button" variant="outline" size="sm" onClick={addVariable} disabled={disabled}>
            <Variable className="size-3.5" />
            Add Variable
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => addSecretBinding(secrets[0])}
            disabled={disabled || secrets.length === 0}>
            <KeyRound className="size-3.5" />
            Bind Secret
          </Button>
          <Button type="button" variant="outline" size="sm" onClick={() => setSecretDialogOpen(true)} disabled={disabled}>
            <Plus className="size-3.5" />
            Create Secret
          </Button>
          <Button type="button" size="sm" onClick={save} disabled={disabled || !hasChanges || replace.isPending}>
            <Save className="size-3.5" />
            Save
          </Button>
        </div>
      </div>

      <ConfigurationEntriesEditor
        entries={entries}
        secrets={secrets}
        isLoading={isLoading}
        emptyText="No resource variables or secrets are defined."
        disabled={disabled}
        onUpdate={updateEntry}
        onRemove={removeEntry}
      />

      <EffectiveConfigurationEntries
        entries={data?.data.effectiveEntries ?? []}
        secrets={secrets}
        resourceScope={scope}
        isLoading={isLoading}
        disabled={disabled}
        onOverrideEntry={overrideEntry}
        onRemoveOverride={removeOverride}
      />

      <CreateSecretDialog
        open={secretDialogOpen}
        input={secretInput}
        isPending={createSecret.isPending}
        onOpenChange={setSecretDialogOpen}
        onInputChange={setSecretInput}
        onCreate={createInternalSecret}
      />
    </div>
  );
};

export const GlobalConfigurationEntriesTab = ({ disabled }: { disabled?: boolean }) => {
  const queryClient = useQueryClient();
  const { data, isLoading } = useRead('getGlobalConfigurationEntries');
  const { data: secretsData } = useRead('listSecretDefinitions');
  const replace = useMutate('replaceGlobalConfigurationEntries');
  const createSecret = useMutate('createInternalSecret');
  const [entries, setEntries] = useState<EditableEntry[]>([]);
  const [secretDialogOpen, setSecretDialogOpen] = useState(false);
  const [secretInput, setSecretInput] = useState<CreateInternalSecretInput>({ name: '', value: '' });

  useEffect(() => {
    if (!data?.data.entries) return;
    setEntries(data.data.entries.map(toEditableEntry));
  }, [data?.data.entries]);

  const hasChanges = useMemo(() => {
    const current = entries.map(toInput);
    const original = (data?.data.entries ?? []).map(toInputFromView);
    return JSON.stringify(current) !== JSON.stringify(original);
  }, [data?.data.entries, entries]);

  const secrets = secretsData?.data.secrets ?? [];

  const updateEntry = (clientId: string, patch: Partial<EditableEntry>) => {
    setEntries((prev) => prev.map((entry) => (entry.clientId === clientId ? { ...entry, ...patch } : entry)));
  };

  const removeEntry = (clientId: string) => {
    setEntries((prev) => prev.filter((entry) => entry.clientId !== clientId));
  };

  const addVariable = () => {
    setEntries((prev) => [
      ...prev,
      {
        clientId: crypto.randomUUID(),
        name: '',
        kind: ConfigurationEntryKind.Variable,
        value: '',
        secretId: null,
        secretDeliveryMode: null,
        targetPath: null,
      },
    ]);
  };

  const addSecretBinding = (secret?: SecretDefinitionView) => {
    setEntries((prev) => [
      ...prev,
      {
        clientId: crypto.randomUUID(),
        name: secret?.name ?? '',
        kind: ConfigurationEntryKind.Secret,
        value: null,
        secretId: secret?.id ?? null,
        secretDeliveryMode: ENV_DELIVERY_MODE,
        targetPath: null,
      },
    ]);
  };

  const save = async () => {
    try {
      await replace.mutateAsync({ data: { entries: entries.map(toInput) } } as any);
      await queryClient.invalidateQueries({ queryKey: ['getGlobalConfigurationEntries'] });
      await queryClient.invalidateQueries({ queryKey: ['getResourceConfigurationEntries'] });
      toast.success('Global variables and secrets saved');
    } catch {
      toast.error(replace.validationErrors ?? 'Failed to save global variables and secrets');
    }
  };

  const createInternalSecret = async () => {
    try {
      const result = await createSecret.mutateAsync({ data: secretInput } as any);
      addSecretBinding(result.data);
      setSecretInput({ name: '', value: '' });
      setSecretDialogOpen(false);
      await queryClient.invalidateQueries({ queryKey: ['listSecretDefinitions', {}] });
      toast.success('Secret created');
    } catch {
      toast.error(createSecret.validationErrors ?? 'Failed to create secret');
    }
  };

  return (
    <div className="flex w-full flex-col gap-4">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <div className="text-sm text-muted-foreground">
          Define global variables and secret bindings inherited by stacks and deployments. Resource values override global
          values.
        </div>
        <div className="flex flex-wrap gap-2">
          <Button type="button" variant="outline" size="sm" onClick={addVariable} disabled={disabled}>
            <Variable className="size-3.5" />
            Add Variable
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => addSecretBinding(secrets[0])}
            disabled={disabled || secrets.length === 0}>
            <KeyRound className="size-3.5" />
            Bind Secret
          </Button>
          <Button type="button" variant="outline" size="sm" onClick={() => setSecretDialogOpen(true)} disabled={disabled}>
            <Plus className="size-3.5" />
            Create Secret
          </Button>
          <Button type="button" size="sm" onClick={save} disabled={disabled || !hasChanges || replace.isPending}>
            <Save className="size-3.5" />
            Save
          </Button>
        </div>
      </div>

      <ConfigurationEntriesEditor
        entries={entries}
        secrets={secrets}
        isLoading={isLoading}
        emptyText="No global variables or secrets are defined."
        disabled={disabled}
        onUpdate={updateEntry}
        onRemove={removeEntry}
      />

      <CreateSecretDialog
        open={secretDialogOpen}
        input={secretInput}
        isPending={createSecret.isPending}
        onOpenChange={setSecretDialogOpen}
        onInputChange={setSecretInput}
        onCreate={createInternalSecret}
      />
    </div>
  );
};

const ConfigurationEntriesEditor = ({
  entries,
  secrets,
  isLoading,
  emptyText,
  disabled,
  onUpdate,
  onRemove,
}: {
  entries: EditableEntry[];
  secrets: SecretDefinitionView[];
  isLoading: boolean;
  emptyText: string;
  disabled?: boolean;
  onUpdate: (clientId: string, patch: Partial<EditableEntry>) => void;
  onRemove: (clientId: string) => void;
}) => {
  if (isLoading) {
    return <div className="rounded-sm border border-dashed px-4 py-6 text-sm text-muted-foreground">Loading...</div>;
  }

  if (!entries.length) {
    return (
      <div className="rounded-sm border border-dashed px-4 py-6 text-center text-sm text-muted-foreground">
        {emptyText}
      </div>
    );
  }

  return (
    <div className="overflow-hidden rounded-sm border">
      <table className="w-full text-sm">
        <thead className="border-b bg-muted/30 text-left text-xs text-muted-foreground">
          <tr>
            <th className="px-3 py-2 font-normal">Name</th>
            <th className="px-3 py-2 font-normal">Type</th>
            <th className="px-3 py-2 font-normal">Value</th>
            <th className="w-10 px-2 py-2" />
          </tr>
        </thead>
        <tbody>
          {entries.map((entry) => (
            <tr key={entry.clientId} className="border-b last:border-0">
              <td className="px-3 py-2 align-top">
                <Input
                  value={entry.name}
                  disabled={disabled}
                  placeholder="DATABASE_URL"
                  onChange={(event) => onUpdate(entry.clientId, { name: event.target.value })}
                />
              </td>
              <td className="px-3 py-2 align-top">
                <div
                  className={cn(
                    'inline-flex h-9 items-center gap-2 rounded-sm border px-3 text-xs text-muted-foreground',
                    entry.kind === ConfigurationEntryKind.Secret ? 'border-dashed' : '',
                  )}>
                  {entry.kind === ConfigurationEntryKind.Secret ? (
                    <>
                      <KeyRound className="size-3.5" />
                      Secret binding
                    </>
                  ) : (
                    <>
                      <Variable className="size-3.5" />
                      Variable
                    </>
                  )}
                </div>
              </td>
              <td className="px-3 py-2 align-top">
                {entry.kind === ConfigurationEntryKind.Variable ? (
                  <Input
                    value={entry.value ?? ''}
                    disabled={disabled}
                    placeholder="Value"
                    onChange={(event) => onUpdate(entry.clientId, { value: event.target.value })}
                  />
                ) : (
                  <Select
                    value={entry.secretId ?? ''}
                    disabled={disabled || secrets.length === 0}
                    onValueChange={(secretId) => onUpdate(entry.clientId, { secretId })}>
                    <SelectTrigger>
                      <SelectValue placeholder={secrets.length ? 'Select secret' : 'No secrets'} />
                    </SelectTrigger>
                    <SelectContent>
                      {secrets.map((secret) => (
                        <SelectItem key={secret.id} value={secret.id}>
                          {secret.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              </td>
              <td className="px-2 py-2 align-top">
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  disabled={disabled}
                  onClick={() => onRemove(entry.clientId)}>
                  <Trash2 className="size-3.5" />
                </Button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

const EffectiveConfigurationEntries = ({
  entries,
  secrets,
  resourceScope,
  isLoading,
  disabled,
  onOverrideEntry,
  onRemoveOverride,
}: {
  entries: ConfigurationEntryView[];
  secrets: SecretDefinitionView[];
  resourceScope: ConfigurationScope.Stack | ConfigurationScope.Deployment;
  isLoading: boolean;
  disabled?: boolean;
  onOverrideEntry?: (entry: ConfigurationEntryView) => void;
  onRemoveOverride?: (name: string) => void;
}) => {
  const localNames = useMemo(
    () => new Set(entries.filter((entry) => entry.scope === resourceScope).map((entry) => entry.name)),
    [entries, resourceScope],
  );
  const globalNames = useMemo(
    () => new Set(entries.filter((entry) => entry.scope === ConfigurationScope.Global).map((entry) => entry.name)),
    [entries],
  );
  const secretNames = useMemo(() => new Map(secrets.map((secret) => [secret.id, secret.name])), [secrets]);
  const sortedEntries = useMemo(
    () =>
      [...entries].sort((left, right) => {
        const byName = left.name.localeCompare(right.name);
        if (byName !== 0) return byName;
        return left.scope === ConfigurationScope.Global ? -1 : 1;
      }),
    [entries],
  );

  if (isLoading || sortedEntries.length === 0) return null;

  const hasActions = Boolean(onOverrideEntry || onRemoveOverride);

  return (
    <div className="flex flex-col gap-2">
      <div>
        <div className="text-sm font-medium">Effective configuration</div>
        <div className="text-xs text-muted-foreground">
          Values available during apply after global inheritance and resource overrides.
        </div>
      </div>
      <div className="overflow-hidden rounded-sm border">
        <table className="w-full text-sm">
          <thead className="border-b bg-muted/30 text-left text-xs text-muted-foreground">
            <tr>
              <th className="px-3 py-2 font-normal">Name</th>
              <th className="px-3 py-2 font-normal">Source</th>
              <th className="px-3 py-2 font-normal">Type</th>
              <th className="px-3 py-2 font-normal">Value</th>
              {hasActions && <th className="w-36 px-3 py-2 font-normal" />}
            </tr>
          </thead>
          <tbody>
            {sortedEntries.map((entry) => {
              const isGlobal = entry.scope === ConfigurationScope.Global;
              const isOverriddenGlobal = isGlobal && localNames.has(entry.name);
              const isResourceOverride = !isGlobal && globalNames.has(entry.name);
              const secretName = entry.secretId ? secretNames.get(entry.secretId) : undefined;

              return (
                <tr key={entry.id} className={cn('border-b last:border-0', isOverriddenGlobal && 'opacity-60')}>
                  <td className="px-3 py-2 align-top font-mono text-xs">{entry.name}</td>
                  <td className="px-3 py-2 align-top">
                    <div className="flex flex-wrap gap-1.5">
                      <Badge variant={isGlobal ? 'secondary' : 'outline'}>{isGlobal ? 'Global' : resourceScope}</Badge>
                      {isResourceOverride && <Badge variant="secondary">Override</Badge>}
                      {isOverriddenGlobal && <Badge variant="outline">Overridden</Badge>}
                    </div>
                  </td>
                  <td className="px-3 py-2 align-top">
                    <span
                      className={cn(
                        'inline-flex items-center gap-1 text-xs text-muted-foreground',
                        entry.kind === ConfigurationEntryKind.Secret && 'font-medium',
                      )}>
                      {entry.kind === ConfigurationEntryKind.Secret ? (
                        <KeyRound className="size-3" />
                      ) : (
                        <Variable className="size-3" />
                      )}
                      {entry.kind}
                    </span>
                  </td>
                  <td className="px-3 py-2 align-top">
                    {entry.kind === ConfigurationEntryKind.Secret ? (
                      <span className="font-mono text-xs text-muted-foreground">
                        {secretName ? `${secretName} ` : ''}
                        <span aria-label="masked secret value">********</span>
                      </span>
                    ) : (
                      <span className="font-mono text-xs">{entry.value}</span>
                    )}
                  </td>
                  {hasActions && (
                    <td className="px-3 py-2 align-top">
                      {isGlobal && !isOverriddenGlobal && onOverrideEntry && (
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          disabled={disabled}
                          onClick={() => onOverrideEntry(entry)}>
                          <Plus className="size-3.5" />
                          Override
                        </Button>
                      )}
                      {isResourceOverride && onRemoveOverride && (
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          disabled={disabled}
                          onClick={() => onRemoveOverride(entry.name)}>
                          <Trash2 className="size-3.5" />
                          Remove
                        </Button>
                      )}
                    </td>
                  )}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
};

const LabeledInput = ({
  label,
  value,
  onChange,
  placeholder,
  type = 'text',
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  type?: string;
}) => (
  <label className="grid gap-1.5 text-sm">
    <span className="text-xs text-muted-foreground">{label}</span>
    <Input type={type} value={value} placeholder={placeholder} onChange={(event) => onChange(event.target.value)} />
  </label>
);

const CreateSecretDialog = ({
  open,
  input,
  isPending,
  onOpenChange,
  onInputChange,
  onCreate,
}: {
  open: boolean;
  input: CreateInternalSecretInput;
  isPending: boolean;
  onOpenChange: (open: boolean) => void;
  onInputChange: (input: CreateInternalSecretInput | ((prev: CreateInternalSecretInput) => CreateInternalSecretInput)) => void;
  onCreate: () => void;
}) => (
  <Dialog open={open} onOpenChange={onOpenChange}>
    <DialogContent>
      <DialogHeader>
        <DialogTitle>Create Secret</DialogTitle>
      </DialogHeader>
      <div className="grid gap-3">
        <LabeledInput
          label="Name"
          value={input.name}
          onChange={(name) => onInputChange((prev) => ({ ...prev, name }))}
          placeholder="API_KEY"
        />
        <LabeledInput
          label="Value"
          type="password"
          value={input.value}
          onChange={(value) => onInputChange((prev) => ({ ...prev, value }))}
          placeholder="Secret value"
        />
      </div>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button type="button" onClick={onCreate} disabled={!input.name.trim() || isPending}>
          Create
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
);

const toEditableEntry = (entry: ConfigurationEntryView): EditableEntry => ({
  clientId: entry.id,
  name: entry.name,
  kind: entry.kind,
  value: entry.value,
  secretId: entry.secretId,
  secretDeliveryMode: entry.secretDeliveryMode,
  targetPath: entry.targetPath,
});

const toInput = (entry: EditableEntry): ConfigurationEntryInput => ({
  name: entry.name,
  kind: entry.kind,
  value: entry.kind === ConfigurationEntryKind.Variable ? (entry.value ?? '') : null,
  secretId: entry.kind === ConfigurationEntryKind.Secret ? entry.secretId : null,
  secretDeliveryMode: entry.kind === ConfigurationEntryKind.Secret ? (entry.secretDeliveryMode ?? ENV_DELIVERY_MODE) : null,
  targetPath: entry.kind === ConfigurationEntryKind.Secret ? (entry.targetPath ?? null) : null,
});

const toInputFromView = (entry: ConfigurationEntryView): ConfigurationEntryInput => ({
  name: entry.name,
  kind: entry.kind,
  value: entry.kind === ConfigurationEntryKind.Variable ? (entry.value ?? '') : null,
  secretId: entry.kind === ConfigurationEntryKind.Secret ? entry.secretId : null,
  secretDeliveryMode: entry.kind === ConfigurationEntryKind.Secret ? (entry.secretDeliveryMode ?? ENV_DELIVERY_MODE) : null,
  targetPath: entry.kind === ConfigurationEntryKind.Secret ? (entry.targetPath ?? null) : null,
});
