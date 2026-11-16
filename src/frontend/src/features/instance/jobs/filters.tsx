import type { ComponentType, ReactNode } from 'react';
import {
  Activity,
  CircleHelp,
  CircleSlash,
  CircleX,
  Clock3,
  Copy,
  Hourglass,
  Layers,
  Square,
  TimerReset,
  Wrench,
} from 'lucide-react';
import { CitadelIcons } from '@/lib/icons';
import { getStateIconAppearance } from '@/components/custom/state-indicator';
import { type JobDisplayState, stateLabel } from './presentation';
import { ResourceFilterChip, ResourceFilterDropdown } from '@/components/custom/resource-filter';
import { CommandGroup, CommandItem } from '@/components/ui/command';
import { Button } from '@/components/ui/button';

const resourceIcons: Record<string, ComponentType<{ className?: string }>> = {
  Alert: CitadelIcons.Alert,
  Build: CitadelIcons.Build,
  Backup: CitadelIcons.BackupPolicy,
  Script: CitadelIcons.Script,
  'Git repository': CitadelIcons.GitRepository,
  Deployment: CitadelIcons.Deployment,
  'Swarm service': CitadelIcons.SwarmService,
  Stack: CitadelIcons.Stack,
  Image: CitadelIcons.Image,
  'Resource copy': Copy,
  Volume: CitadelIcons.Volume,
  Container: CitadelIcons.Container,
  Updates: CitadelIcons.Updates,
  Platform: CitadelIcons.Platform,
  Maintenance: Wrench,
  Core: CitadelIcons.Instance,
};

export function jobResourceOption(name: string) {
  const Icon = resourceIcons[name] ?? Layers;
  return { value: name, label: name, icon: <Icon className="size-3.5" /> };
}

const stateIcons: Record<JobDisplayState, ComponentType<{ className?: string }>> = {
  running: Activity,
  waiting: Clock3,
  notStarted: Hourglass,
  backingOff: TimerReset,
  failed: CircleX,
  unavailable: CircleHelp,
  disabled: CircleSlash,
  stopped: Square,
};

export function jobStateOption(value: JobDisplayState) {
  const Icon = stateIcons[value];
  return {
    value,
    label: stateLabel(value),
    icon: <Icon className={`size-3.5 ${getStateIconAppearance(value, 'job').className}`} />,
  };
}

export function JobFilter({
  label,
  icon: Icon,
  options,
  selected,
  onChange,
}: {
  label: string;
  icon: ComponentType<{ className?: string }>;
  options: { value: string; label: string; icon?: ReactNode }[];
  selected: string[];
  onChange: (values: string[]) => void;
}) {
  return (
    <div className="flex min-w-0 flex-wrap items-center gap-2">
      {selected.map((value) => {
        const option = options.find((option) => option.value === value);
        const name = option?.label ?? value;
        return (
          <ResourceFilterChip
            key={value}
            label={`Remove ${name} ${label.toLowerCase()} filter`}
            icon={option?.icon ?? <Icon className="size-3.5" />}
            onClick={() => onChange(selected.filter((item) => item !== value))}>
            {name}
          </ResourceFilterChip>
        );
      })}
      <ResourceFilterDropdown
        label={`${label} Filter`}
        icon={Icon}
        searchPlaceholder={`Search ${label.toLowerCase()}s…`}
        emptyMessage={`No matching ${label.toLowerCase()}s.`}
        footer={
          selected.length > 0 && (
            <div className="border-t p-1">
              <Button variant="ghost" size="sm" className="w-full justify-start" onClick={() => onChange([])}>
                Clear {label.toLowerCase()} filters
              </Button>
            </div>
          )
        }>
        <CommandGroup>
          {options
            .filter((option) => !selected.includes(option.value))
            .map((option) => (
              <CommandItem
                key={option.value}
                value={option.label}
                onSelect={() => onChange([...selected, option.value])}>
                {option.icon ?? <Icon className="size-3.5" />}
                {option.label}
              </CommandItem>
            ))}
        </CommandGroup>
      </ResourceFilterDropdown>
    </div>
  );
}
