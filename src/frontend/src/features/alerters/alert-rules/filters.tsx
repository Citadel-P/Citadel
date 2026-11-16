import { useMemo, type ComponentType, type ReactNode } from 'react';
import { useSearchParams } from 'react-router';
import { Bell, TriangleAlert } from 'lucide-react';
import { AlertSeverity, type AlertRuleListItem } from '@/api/generated/api.types';
import { ResourceFilterChip, ResourceFilterDropdown } from '@/components/custom/resource-filter';
import { CommandGroup, CommandItem } from '@/components/ui/command';
import { getStateIconAppearance } from '@/components/custom/state-indicator';
import type { RuleFilters } from './presentation';

type FilterOption = { value: string; label: string; icon?: ReactNode };

export function useAlertRuleFilters() {
  const [params, setParams] = useSearchParams();
  const value = useMemo<RuleFilters>(
    () => ({
      severities: Object.values(AlertSeverity).filter((severity) => params.getAll('severity').includes(severity)),
      channels: params.getAll('channel'),
    }),
    [params],
  );
  const onChange = (value: RuleFilters) =>
    setParams((current) => {
      const next = new URLSearchParams(current);
      next.delete('severity');
      next.delete('channel');
      value.severities.forEach((severity) => next.append('severity', severity));
      value.channels.forEach((channel) => next.append('channel', channel));
      return next;
    });
  return { value, onChange };
}

export function AlertRuleFilters({ items }: { items: AlertRuleListItem[] }) {
  const { value, onChange } = useAlertRuleFilters();
  const channels = [
    ...new Map(items.flatMap((rule) => rule.channels.map((channel) => [channel.id, channel] as const))).values(),
  ];
  const fields: {
    key: keyof RuleFilters;
    label: string;
    Icon: ComponentType<{ className?: string }>;
    options: FilterOption[];
  }[] = [
    {
      key: 'severities',
      label: 'Severity',
      Icon: TriangleAlert,
      options: Object.values(AlertSeverity).map((severity) => ({
        value: severity,
        label: severity,
        icon: <TriangleAlert className={`size-3.5 ${getStateIconAppearance(severity, 'alertSeverity').className}`} />,
      })),
    },
    {
      key: 'channels',
      label: 'Channel',
      Icon: Bell,
      options: channels.map((channel) => ({ value: channel.id, label: channel.name })),
    },
  ];
  return (
    <div role="region" aria-label="Alert rule filters" className="flex flex-wrap items-center gap-2">
      {fields.map(({ key, label, Icon, options }) => (
        <div key={key} className="flex flex-wrap items-center gap-2">
          {value[key].map((selected) => {
            const option = options.find((option) => option.value === selected);
            return (
              <ResourceFilterChip
                key={selected}
                label={`Remove ${option?.label ?? selected} ${label.toLowerCase()} filter`}
                icon={option?.icon}
                onClick={() => onChange({ ...value, [key]: value[key].filter((id) => id !== selected) })}>
                {option?.label ?? selected}
              </ResourceFilterChip>
            );
          })}
          <ResourceFilterDropdown
            label={`${label} Filter`}
            icon={Icon}
            searchPlaceholder={`Search ${label.toLowerCase()}…`}
            emptyMessage="No matching options.">
            <CommandGroup>
              {options
                .filter((option) => !value[key].includes(option.value))
                .map((option) => (
                  <CommandItem
                    key={option.value}
                    value={option.label}
                    onSelect={() => onChange({ ...value, [key]: [...value[key], option.value] })}>
                    {option.icon ?? <Icon className="size-3.5" />}
                    {option.label}
                  </CommandItem>
                ))}
            </CommandGroup>
          </ResourceFilterDropdown>
        </div>
      ))}
    </div>
  );
}
