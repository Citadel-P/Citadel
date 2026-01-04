import { useMemo, useState, useLayoutEffect, useRef, useEffect } from 'react';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { AutoUpdateStatus, UpdateBehavior } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Command, CommandInput, CommandList, CommandGroup, CommandItem, CommandEmpty } from '@/components/ui/command';
import { Check, ChevronDown, LucideIcon, Tags, X } from 'lucide-react';
import { cn, filterBySplit } from '@/lib/utils';
import { PluralResourceMap, ResourceType } from '@/api/types';
import { useMeasuredWidth, useRead } from '@/lib/hooks';
import { useResourceFilter } from '@/lib/atoms';
import { Badge } from '../ui/badge';
import { MultiSelect, MultiSelectOption } from '../ui/multi-select';
import {
  Bell,
  CircleCheck,
  CircleQuestionMark,
  CircleX,
  RefreshCcw,
  RefreshCcwDot,
  RefreshCwOff,
  SquareArrowUp,
} from 'lucide-react';

export function ResourceSelectorField<T extends { id: string; name: string }>({
  type,
  selected,
  onSelect,
  disabled,
  align = 'start',
  placeholder,
  className,
  platformId,
}: {
  type: ResourceType;
  selected?: T | string | undefined;
  onSelect?: (item: T | undefined) => void;
  disabled?: boolean;
  align?: 'start' | 'center' | 'end';
  placeholder?: string;
  className?: string;
  platformId?: string;
}) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');

  const { ref: triggerRef, width: contentWidth, measure } = useMeasuredWidth(400);

  const resourceName = PluralResourceMap[type];
  const [filter, setFilter] = useResourceFilter<{ item: T }>(type);

  const read = useRead(`list${resourceName}`, { platformId });
  const items = (Object.values(read.data?.data ?? {}).at(0) as T[]) ?? [];

  const selectedItem =
    filter?.item ?? (typeof selected === 'string' ? items.find((i) => i.id === selected) : selected) ?? undefined;

  const filtered = filterBySplit(items, search, (i) => i.name).sort((a, b) => a.name.localeCompare(b.name));

  const handleSelect = (item: T | undefined) => {
    setFilter(item ? { item } : null);
    onSelect?.(item);
    setOpen(false);
  };

  useLayoutEffect(() => {
    if (open) measure();
  }, [open, selectedItem, measure]);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          ref={triggerRef as any}
          disabled={disabled}
          variant="ghost"
          role="combobox"
          aria-expanded={open}
          data-placeholder={selectedItem ? undefined : ''}
          className={cn(
            'flex justify-between gap-2 w-full max-w-[400px] font-normal data-[placeholder]:text-muted-foreground text-sm bg-background hover:bg-background shadow-xs border',
            className,
          )}>
          {selectedItem?.name ?? placeholder}
          <ChevronDown className="h-4 w-4 opacity-60" />
        </Button>
      </PopoverTrigger>

      <PopoverContent
        align={align}
        className="w-full max-w-[400px] p-0 bg-background"
        style={contentWidth ? { width: `${contentWidth}px` } : undefined}>
        <Command shouldFilter={false} defaultValue={selectedItem?.name ?? '__none__'}>
          <CommandInput placeholder={`Search ${PluralResourceMap[type]}`} value={search} onValueChange={setSearch} />

          <CommandList>
            <CommandEmpty>No results found.</CommandEmpty>

            <CommandGroup>
              {!search && (
                <CommandItem
                  value="__none__"
                  onSelect={() => handleSelect(undefined)}
                  className="flex items-center justify-between cursor-pointer my-0.5 px-2 py-2 rounded-sm">
                  <span>None</span>
                  <Check className={cn('h-4 w-4 transition-opacity', !selectedItem ? 'opacity-100' : 'opacity-0')} />
                </CommandItem>
              )}

              {filtered.map((item) => {
                const isSelected = selectedItem?.id === item.id;

                return (
                  <CommandItem
                    key={item.id}
                    value={item.name}
                    onSelect={() => handleSelect(item)}
                    className="flex items-center justify-between cursor-pointer my-0.5 px-2 py-2 rounded-sm">
                    <span>{item.name}</span>
                    <Check className={cn('h-4 w-4 transition-opacity', isSelected ? 'opacity-100' : 'opacity-0')} />
                  </CommandItem>
                );
              })}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}

export function MultiResourceSelectorField<T extends { id: string; name: string }>({
  type,
  selected,
  onSelect,
  disabled,
  placeholder,
  platformId,
  className,
}: {
  type: ResourceType;
  selected?: string[] | T[];
  onSelect?: (items: T[]) => void;
  disabled?: boolean;
  align?: 'start' | 'center' | 'end';
  placeholder?: string;
  platformId?: string;
  className?: string;
}) {
  const resourceName = PluralResourceMap[type];

  const read = useRead(`list${resourceName}`, { platformId });
  const items = (Object.values(read.data?.data ?? {}).at(0) as T[]) ?? [];

  const options: MultiSelectOption[] = useMemo(
    () =>
      items
        .map((item) => ({
          label: item.name,
          value: item.id,
        }))
        .sort((a, b) => a.label.localeCompare(b.label)),
    [items],
  );

  const selectedIds = useMemo(() => {
    if (!selected) return [];
    return selected.map((s) => (typeof s === 'string' ? s : s.id));
  }, [selected]);

  const handleValueChange = (newIds: string[]) => {
    const newSelectedItems = items.filter((item) => newIds.includes(item.id));
    onSelect?.(newSelectedItems);
  };

  return (
    <MultiSelect
      options={options}
      defaultValue={selectedIds}
      onValueChange={handleValueChange}
      placeholder={read.isLoading ? 'Loading...' : (placeholder ?? `Select ${resourceName}...`)}
      disabled={disabled || read.isLoading}
      maxWidth="400px"
      className={cn(
        'flex justify-between w-full text-sm bg-background font-normal hover:bg-background shadow-xs border',
        className,
      )}
      searchable={true}
      maxCount={3}
      resetOnDefaultValueChange={true}
      animation={0}
    />
  );
}

export const DockerLabelsSection = ({ labels }: { labels: Record<string, string> | undefined }) => {
  if (!labels) return null;
  const entries = Object.entries(labels);
  if (entries.length === 0) return null;
  return (
    <Section title="Labels" Icon={Tags}>
      <div className="flex gap-2 flex-wrap">
        <KeyPairEntries items={labels} />
      </div>
    </Section>
  );
};

export const KeyPairEntries = ({ items }: { items: Record<string, string> | undefined }) => {
  if (!items) return null;
  const entries = Object.entries(items);
  if (entries.length === 0) return null;
  return (
    <div className="flex gap-2 flex-wrap">
      {entries.map(([key, value]) => (
        <Badge key={key} variant="secondary" className="flex gap-1">
          <span className="text-muted-foreground">{key}</span>
          <span className="text-muted-foreground">=</span>
          <span title={value} className="font-medium text-nowrap max-w-[200px] overflow-hidden text-ellipsis">
            {value}
          </span>
        </Badge>
      ))}
    </div>
  );
};

export const Section = ({ Icon, title, children }: { Icon: LucideIcon; title: string; children: React.ReactNode }) => {
  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-row items-center gap-2">
        <Icon width={14} height={14} className="text-muted-foreground" />
        <div className="text-sm font-semibold text-muted-foreground leading-none">{title}</div>
      </div>
      {children}
    </div>
  );
};
export const UPDATE_BEHAVIOR_UI: Record<
  UpdateBehavior,
  {
    label: string;
    Icon: React.ComponentType<{ width?: number; height?: number; className?: string }>;
    className: string;
  }
> = {
  [UpdateBehavior.Disabled]: {
    label: 'Manual',
    Icon: RefreshCwOff,
    className: 'text-foreground/80',
  },
  [UpdateBehavior.AutoDeploy]: {
    label: 'Auto',
    Icon: RefreshCcw,
    className: 'text-green-400',
  },
  [UpdateBehavior.Notify]: {
    label: 'Notify',
    Icon: Bell,
    className: 'text-yellow-400',
  },
};

export const UPDATE_STATUS_UI: Record<
  AutoUpdateStatus,
  {
    label: string;
    Icon: React.ComponentType<{ width?: number; height?: number; className?: string }>;
    className: string;
  }
> = {
  [AutoUpdateStatus.Unknown]: {
    label: 'Unknown',
    Icon: CircleQuestionMark,
    className: 'text-foreground/80',
  },
  [AutoUpdateStatus.UpToDate]: {
    label: 'Up to date',
    Icon: CircleCheck,
    className: 'text-green-400',
  },
  [AutoUpdateStatus.Failed]: {
    label: 'Failed',
    Icon: CircleX,
    className: 'text-orange-400',
  },
  [AutoUpdateStatus.UpdateAvailable]: {
    label: 'Update Available',
    Icon: SquareArrowUp,
    className: 'text-blue-400',
  },
  [AutoUpdateStatus.Updating]: {
    label: 'Update Available',
    Icon: RefreshCcwDot,
    className: 'text-blue-400',
  },
};

export const AutoUpdateIcon = ({ updateBehavior }: { updateBehavior: UpdateBehavior }) => {
  const { Icon, className } = UPDATE_BEHAVIOR_UI[updateBehavior];

  return <Icon width={14} height={14} className={className} />;
};

export const UpdateStatusIcon = ({ updateStatus }: { updateStatus: AutoUpdateStatus }) => {
  const { Icon, className } = UPDATE_STATUS_UI[updateStatus];

  return <Icon width={14} height={14} className={className} />;
};

interface LogViewerProps {
  logs: string | string[];
  autoScroll?: boolean;
  maxLines?: number;
  className?: string;
}

export const LogViewer = ({ logs, autoScroll = true, className }: LogViewerProps) => {
  const scrollRef = useRef<HTMLPreElement>(null);
  const [isAtBottom, setIsAtBottom] = useState(true);

  const handleScroll = () => {
    if (!scrollRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollRef.current;
    const atBottom = scrollHeight - scrollTop <= clientHeight + 50;
    setIsAtBottom(atBottom);
  };

  useEffect(() => {
    if (autoScroll && isAtBottom && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs, autoScroll, isAtBottom]);
  const content = Array.isArray(logs) ? logs.join('\n') : logs;
  return (
    <pre
      ref={scrollRef}
      onScroll={handleScroll}
      className={cn("p-4 max-h-[600px] rounded-sm border shadow-xs inline-block w-full overflow-auto bg-transparent text-xs", className)}
      style={{ scrollBehavior: 'auto' }}>
      {content || <span className="text-zinc-500 italic">Waiting for output...</span>}
    </pre>
  );
};
