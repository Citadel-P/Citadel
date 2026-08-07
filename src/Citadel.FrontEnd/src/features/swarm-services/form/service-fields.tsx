import { Plus, Trash2 } from 'lucide-react';
import {
  SwarmServiceConfigReference,
  SwarmServiceMount,
  SwarmServiceMountKind,
  SwarmServicePort,
  SwarmServicePortPublishMode,
  SwarmServiceSecretReference,
} from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';

type SwarmNamedResource = { id: string; name: string };

export const ServicePortsField = ({
  value,
  onChange,
  disabled,
}: {
  value: SwarmServicePort[];
  onChange: (value: SwarmServicePort[]) => void;
  disabled?: boolean;
}) => {
  const update = (index: number, patch: Partial<SwarmServicePort>) => {
    const next = [...value];
    next[index] = { ...next[index], ...patch };
    onChange(next);
  };

  return (
    <div className="space-y-2">
      {value.map((port, index) => (
        <div key={index} className="grid grid-cols-[1fr_1fr_7rem_8rem_auto] gap-2">
          <Input
            aria-label={`Target port ${index + 1}`}
            type="number"
            min={1}
            max={65535}
            value={port.targetPort}
            disabled={disabled}
            placeholder="Target"
            onChange={(event) => update(index, { targetPort: Number(event.target.value) })}
          />
          <Input
            aria-label={`Published port ${index + 1}`}
            type="number"
            min={1}
            max={65535}
            value={port.publishedPort ?? ''}
            disabled={disabled}
            placeholder="Published"
            onChange={(event) =>
              update(index, { publishedPort: event.target.value ? Number(event.target.value) : null })
            }
          />
          <Select
            value={port.protocol ?? 'tcp'}
            disabled={disabled}
            onValueChange={(protocol) => update(index, { protocol })}>
            <SelectTrigger aria-label={`Protocol ${index + 1}`}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="tcp">TCP</SelectItem>
              <SelectItem value="udp">UDP</SelectItem>
              <SelectItem value="sctp">SCTP</SelectItem>
            </SelectContent>
          </Select>
          <Select
            value={port.publishMode ?? SwarmServicePortPublishMode.Ingress}
            disabled={disabled}
            onValueChange={(publishMode: SwarmServicePortPublishMode) => update(index, { publishMode })}>
            <SelectTrigger aria-label={`Publish mode ${index + 1}`}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={SwarmServicePortPublishMode.Ingress}>Ingress</SelectItem>
              <SelectItem value={SwarmServicePortPublishMode.Host}>Host</SelectItem>
            </SelectContent>
          </Select>
          <Button
            type="button"
            variant="destructive"
            size="icon"
            disabled={disabled}
            onClick={() => onChange(value.filter((_, itemIndex) => itemIndex !== index))}>
            <Trash2 className="h-4 w-4" />
            <span className="sr-only">Remove port</span>
          </Button>
        </div>
      ))}
      <Button
        type="button"
        variant="outline"
        size="sm"
        disabled={disabled}
        onClick={() =>
          onChange([
            ...value,
            { targetPort: 80, publishedPort: null, protocol: 'tcp', publishMode: SwarmServicePortPublishMode.Ingress },
          ])
        }>
        <Plus className="mr-2 h-4 w-4" />
        Add port
      </Button>
    </div>
  );
};

export const ServiceMountsField = ({
  value,
  onChange,
  disabled,
}: {
  value: SwarmServiceMount[];
  onChange: (value: SwarmServiceMount[]) => void;
  disabled?: boolean;
}) => {
  const update = (index: number, patch: Partial<SwarmServiceMount>) => {
    const next = [...value];
    next[index] = { ...next[index], ...patch };
    onChange(next);
  };

  return (
    <div className="space-y-2">
      {value.map((mount, index) => (
        <div key={index} className="grid grid-cols-[8rem_1fr_1fr_6rem_auto] items-center gap-2">
          <Select
            value={mount.kind}
            disabled={disabled}
            onValueChange={(kind: SwarmServiceMountKind) => update(index, { kind })}>
            <SelectTrigger aria-label={`Mount type ${index + 1}`}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={SwarmServiceMountKind.Volume}>Volume</SelectItem>
              <SelectItem value={SwarmServiceMountKind.Bind}>Bind</SelectItem>
            </SelectContent>
          </Select>
          <Input
            aria-label={`Mount source ${index + 1}`}
            value={mount.source}
            disabled={disabled}
            placeholder="Source"
            onChange={(event) => update(index, { source: event.target.value })}
          />
          <Input
            aria-label={`Mount target ${index + 1}`}
            value={mount.target}
            disabled={disabled}
            placeholder="/container/path"
            onChange={(event) => update(index, { target: event.target.value })}
          />
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <Switch
              aria-label={`Read-only mount ${index + 1}`}
              checked={mount.readOnly ?? false}
              disabled={disabled}
              onCheckedChange={(readOnly) => update(index, { readOnly })}
            />
            Read only
          </div>
          <Button
            type="button"
            variant="destructive"
            size="icon"
            disabled={disabled}
            onClick={() => onChange(value.filter((_, itemIndex) => itemIndex !== index))}>
            <Trash2 className="h-4 w-4" />
            <span className="sr-only">Remove mount</span>
          </Button>
        </div>
      ))}
      <Button
        type="button"
        variant="outline"
        size="sm"
        disabled={disabled}
        onClick={() =>
          onChange([...value, { kind: SwarmServiceMountKind.Volume, source: '', target: '', readOnly: false }])
        }>
        <Plus className="mr-2 h-4 w-4" />
        Add mount
      </Button>
    </div>
  );
};

export const ServiceReferencesField = ({
  kind,
  resources,
  value,
  onChange,
  disabled,
}: {
  kind: 'secret' | 'config';
  resources: SwarmNamedResource[];
  value: Array<SwarmServiceSecretReference | SwarmServiceConfigReference>;
  onChange: (value: Array<SwarmServiceSecretReference | SwarmServiceConfigReference>) => void;
  disabled?: boolean;
}) => {
  const idKey = kind === 'secret' ? 'secretId' : 'configId';
  const nameKey = kind === 'secret' ? 'secretName' : 'configName';
  const readId = (reference: SwarmServiceSecretReference | SwarmServiceConfigReference) =>
    kind === 'secret'
      ? (reference as SwarmServiceSecretReference).secretId
      : (reference as SwarmServiceConfigReference).configId;
  const updateResource = (index: number, resourceId: string) => {
    const resource = resources.find((item) => item.id === resourceId);
    if (!resource) return;
    const next = [...value];
    next[index] = { [idKey]: resource.id, [nameKey]: resource.name, targetName: resource.name } as
      | SwarmServiceSecretReference
      | SwarmServiceConfigReference;
    onChange(next);
  };
  const updateTarget = (index: number, targetName: string) => {
    const next = [...value];
    next[index] = { ...next[index], targetName };
    onChange(next);
  };
  const selected = new Set(value.map(readId));

  return (
    <div className="space-y-2">
      {value.map((reference, index) => (
        <div key={`${readId(reference)}-${index}`} className="flex flex-col gap-2 sm:flex-row sm:items-center">
          <Select
            value={readId(reference)}
            disabled={disabled}
            onValueChange={(resourceId) => updateResource(index, resourceId)}>
            <SelectTrigger className="min-h-10 w-full min-w-0 max-w-100 sm:flex-1" aria-label={`${kind} ${index + 1}`}>
              <SelectValue placeholder={`Select ${kind}`} />
            </SelectTrigger>
            <SelectContent className="bg-background">
              {resources.map((resource) => (
                <SelectItem
                  key={resource.id}
                  value={resource.id}
                  disabled={selected.has(resource.id) && resource.id !== readId(reference)}>
                  {resource.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <div className="flex w-full min-w-0 items-center gap-2 sm:flex-1">
            <Input
              className="min-w-0 max-w-100 flex-1"
              aria-label={`${kind} target ${index + 1}`}
              value={reference.targetName}
              disabled={disabled}
              placeholder="Target filename"
              onChange={(event) => updateTarget(index, event.target.value)}
            />
            <Button
              className="shrink-0"
              type="button"
              variant="destructive"
              size="icon"
              disabled={disabled}
              onClick={() => onChange(value.filter((_, itemIndex) => itemIndex !== index))}>
              <Trash2 className="h-4 w-4" />
              <span className="sr-only">Remove {kind}</span>
            </Button>
          </div>
        </div>
      ))}
      <Button
        type="button"
        variant="outline"
        size="sm"
        className="w-50 min-h-9"
        disabled={disabled || resources.every((resource) => selected.has(resource.id))}
        onClick={() => {
          const resource = resources.find((item) => !selected.has(item.id));
          if (!resource) return;
          onChange([
            ...value,
            { [idKey]: resource.id, [nameKey]: resource.name, targetName: resource.name } as
              | SwarmServiceSecretReference
              | SwarmServiceConfigReference,
          ]);
        }}>
        <Plus className="mr-2 h-4 w-4" />
        Add {kind}
      </Button>
    </div>
  );
};
