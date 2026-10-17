import { useMemo } from 'react';
import { useSearchParams } from 'react-router';
import { Cable, Globe, Network, type LucideIcon } from 'lucide-react';
import type { NetworkView } from '@/api/generated/api.types';
import { ResourceFilterChip, ResourceFilterDropdown } from '@/components/custom/resource-filter';
import { CommandGroup, CommandItem } from '@/components/ui/command';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';

const drivers = ['bridge', 'overlay', 'macvlan', 'ipvlan', 'host', 'null'] as const;
const scopes = ['local', 'swarm', 'global'] as const;
const options = {
  driver: [
    ...drivers.map((value) => ({ value, label: value === 'null' ? 'none (null)' : value })),
    { value: 'other', label: 'Other drivers' },
  ],
  scope: [...scopes.map((value) => ({ value, label: value })), { value: 'other', label: 'Other scopes' }],
  usage: [
    { value: 'attached', label: 'Has visible attachments' },
    { value: 'in-use', label: 'In use' },
    { value: 'not-reported', label: 'No reported use' },
    { value: 'unknown', label: 'Unknown' },
  ],
};
export const networkFilterParams = ['driver', 'scope', 'usage', 'system'];
type FilterKey = keyof typeof options;

export function readNetworkFilters(params: URLSearchParams) {
  const selected = (key: FilterKey) =>
    options[key].filter(({ value }) => params.getAll(key).includes(value)).map(({ value }) => value);
  return {
    driver: selected('driver'),
    scope: selected('scope'),
    usage: selected('usage'),
    hideSystem: params.get('system') === 'hide',
  };
}

export function applyNetworkFilters(items: NetworkView[], params: URLSearchParams): NetworkView[] {
  const filters = readNetworkFilters(params);
  if (!filters.driver.length && !filters.scope.length && !filters.usage.length && !filters.hideSystem) return items;
  return items.filter((network) => {
    if (filters.hideSystem && network.isSystem) return false;
    const driver = drivers.some((value) => value === network.driver) ? network.driver : 'other';
    const scope = scopes.some((value) => value === network.scope) ? network.scope : 'other';
    if (filters.driver.length && !filters.driver.includes(driver)) return false;
    if (filters.scope.length && !filters.scope.includes(scope)) return false;
    if (!filters.usage.length) return true;
    // Empty actor-filtered endpoints never override Docker's usage flag. Saved
    // references are desired configuration, not evidence of an attachment.
    const attached =
      Object.keys(network.containers ?? {}).length > 0 ||
      !!network.usage?.entries.some((entry) => entry.relationship === 'Attachment');
    const unknown = network.isStale || typeof network.inUse !== 'boolean';
    return filters.usage.some((value) => {
      if (value === 'unknown') return unknown;
      if (unknown) return false;
      if (value === 'attached') return attached;
      if (value === 'in-use') return network.inUse || attached;
      return !network.inUse && !attached;
    });
  });
}

export function useNetworkFilters(items: NetworkView[]) {
  const [params] = useSearchParams();
  // Exclude view/focus parameters so switching views does not rebuild the list.
  const selection = new URLSearchParams(
    networkFilterParams.flatMap((key) => params.getAll(key).map((value) => [key, value])),
  ).toString();
  return useMemo(() => applyNetworkFilters(items, new URLSearchParams(selection)), [items, selection]);
}

function NetworkCategoryFilter({ name, label, icon }: { name: FilterKey; label: string; icon: LucideIcon }) {
  const [params, setParams] = useSearchParams();
  const selected = readNetworkFilters(params)[name];
  const update = (values: string[]) =>
    setParams((current) => {
      const next = new URLSearchParams(current);
      next.delete(name);
      values.forEach((value) => next.append(name, value));
      return next;
    });
  return (
    <div className="flex flex-wrap items-center gap-2">
      {options[name]
        .filter(({ value }) => selected.includes(value))
        .map(({ value, label: optionLabel }) => (
          <ResourceFilterChip
            key={value}
            label={`Remove ${optionLabel} ${name} filter`}
            onClick={() => update(selected.filter((item) => item !== value))}>
            {optionLabel}
          </ResourceFilterChip>
        ))}
      <ResourceFilterDropdown
        label={label}
        icon={icon}
        searchPlaceholder={`Search ${name} filters…`}
        emptyMessage="No more filters."
        footer={
          name === 'usage' ? (
            <p className="border-t p-3 text-xs text-muted-foreground">
              No reported use does not mean safe to delete. Visible attachments depend on available observations and
              your access.
            </p>
          ) : undefined
        }>
        <CommandGroup>
          {options[name]
            .filter(({ value }) => !selected.includes(value))
            .map(({ value, label: optionLabel }) => (
              <CommandItem
                key={value}
                value={optionLabel}
                onSelect={() => update([...selected, value])}
                className="cursor-pointer">
                {optionLabel}
              </CommandItem>
            ))}
        </CommandGroup>
      </ResourceFilterDropdown>
    </div>
  );
}

export function NetworkFilters() {
  const [params, setParams] = useSearchParams();
  const filters = readNetworkFilters(params);
  const active = filters.driver.length || filters.scope.length || filters.usage.length || filters.hideSystem;
  return (
    <>
      <NetworkCategoryFilter name="driver" label="Driver" icon={Network} />
      <NetworkCategoryFilter name="scope" label="Scope" icon={Globe} />
      <NetworkCategoryFilter name="usage" label="Usage" icon={Cable} />
      <div className="inline-flex h-9 items-center gap-2 rounded-sm border bg-background px-2.5 text-xs text-muted-foreground">
        <Switch
          id="show-system-networks"
          checked={!filters.hideSystem}
          aria-label="Show system networks"
          onCheckedChange={(checked) =>
            setParams((current) => {
              const next = new URLSearchParams(current);
              if (checked) next.delete('system');
              else next.set('system', 'hide');
              return next;
            })
          }
        />
        <Label htmlFor="show-system-networks" className="whitespace-nowrap text-xs font-normal">
          Show system networks
        </Label>
      </div>
      {!!active && (
        <Button
          variant="ghost"
          size="sm"
          onClick={() =>
            setParams((current) => {
              const next = new URLSearchParams(current);
              networkFilterParams.forEach((key) => next.delete(key));
              return next;
            })
          }>
          Clear filters
        </Button>
      )}
    </>
  );
}
