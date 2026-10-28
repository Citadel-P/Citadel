import {
  ActivityResourceType,
  ActorType,
  AlertResourceType,
  AutoUpdateStatus,
  ContainerStateStatus,
  ContainerStatView,
  LookupResourceType,
  PlatformStatus,
  UpdateBehavior,
} from '@/api/generated/api.types';
import { PluralResourceMap } from '@/api/types';
import { Button } from '@/components/ui/button';
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from '@/components/ui/command';
import { DataTable } from '@/components/ui/data-table';
import {
  Pagination,
  PaginationContent,
  PaginationEllipsis,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from '@/components/ui/pagination';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Select, SelectContent, SelectItem, SelectSeparator, SelectTrigger, SelectValue } from '@/components/ui/select';
import { byteTransform } from '@/lib/bytes.helper';
import { useMeasuredWidth, useRead } from '@/lib/hooks';
import { truncate } from '@/lib/truncate';
import { cn, filterBySplit, formatId, normalizeDockerId, toFixedNumber } from '@/lib/utils';
import { StockFeatures, ColumnDef } from '@tanstack/react-table';
import {
  ArrowRight,
  ArrowUpCircle,
  Bell,
  Bot,
  Boxes,
  Cable,
  CalendarClock,
  Check,
  ChevronDown,
  CircleCheck,
  CircleQuestionMark,
  CircleX,
  Database,
  GitBranch,
  Hammer,
  HardDrive,
  Info,
  Layers,
  LucideIcon,
  Megaphone,
  Network,
  Plus,
  RefreshCcw,
  RefreshCcwDot,
  RefreshCwOff,
  Rocket,
  Server,
  ServerPlus,
  Settings,
  Shield,
  Tags,
  Unlink,
  User,
  Users,
  Workflow,
  X,
} from 'lucide-react';
import { useCallback, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from 'react';
import { Link } from 'react-router';
import { Badge } from '../ui/badge';
import { CardDescription, CardHeader, CardTitle } from '../ui/card';
import { MultiSelect, MultiSelectOption } from '../ui/multi-select';
import { useFormFieldAccessibility } from './form-field-accessibility';
import { StateIndicator } from './state-indicator';

export const OverflowCountBadge = ({
  count,
  className,
  title,
}: {
  count: number;
  className?: string;
  title?: string;
}) => {
  if (count <= 0) return null;

  return (
    <span
      className={cn('rounded-sm bg-accent/60 px-1.5 py-0.5 text-[11px] text-muted-foreground', className)}
      title={title}>
      +{count}
    </span>
  );
};

export const StatsPanelHeader = ({
  title,
  description,
  controls,
  children,
}: {
  title: ReactNode;
  description: ReactNode;
  controls?: ReactNode;
  children?: ReactNode;
}) => (
  <CardHeader className="flex flex-col items-stretch border-b p-0! xl:flex-row">
    <div className="flex min-w-0 flex-1 flex-col justify-center gap-3 px-6 py-4 lg:flex-row lg:items-center lg:justify-between lg:gap-4 xl:py-0">
      <div className="min-w-0 space-y-1">
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </div>
      {controls && <div className="shrink-0">{controls}</div>}
    </div>
    {children && <div className="flex flex-wrap xl:shrink-0">{children}</div>}
  </CardHeader>
);

export const StatsSummaryItem = ({ label, value }: { label: ReactNode; value: ReactNode }) => (
  <div className="flex flex-1 flex-col justify-center gap-1 border-t px-6 py-4 text-left even:border-l xl:border-t-0 xl:border-l xl:px-8 xl:py-6">
    <span className="text-xs text-muted-foreground">{label}</span>
    <span className="text-sm text-foreground font-medium leading-none">{value}</span>
  </div>
);

export const IntegrationCard = ({
  title,
  subtitle,
  icon,
  footerLeft,
  footerRight,
  disabled = false,
  onEdit,
}: {
  title: ReactNode;
  subtitle: ReactNode;
  icon: ReactNode;
  footerLeft: ReactNode;
  footerRight: ReactNode;
  disabled?: boolean;
  onEdit: () => void;
}) => {
  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (disabled) return;
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onEdit();
    }
  };

  return (
    <div
      role="button"
      tabIndex={disabled ? -1 : 0}
      onClick={disabled ? undefined : onEdit}
      onKeyDown={handleKeyDown}
      aria-disabled={disabled}
      className={cn(
        'group relative bg-card rounded-lg border border-border p-(--surface-padding) hover:border-ring/50 hover:shadow-xs transition-all flex flex-col justify-between h-35',
        disabled ? 'hover:cursor-not-allowed opacity-70' : 'cursor-pointer',
      )}>
      <div className="flex items-start justify-between">
        <div className="flex min-w-0 items-center gap-3">
          {icon}
          <div className="min-w-0">
            <h3 className="truncate text-sm font-semibold">{title}</h3>
            <p className="truncate text-xs text-muted-foreground">{subtitle}</p>
          </div>
        </div>
      </div>

      <div className="mt-4 flex items-center justify-between border-t border-muted pt-4">
        {footerLeft}
        <div
          onClick={(event) => event.stopPropagation()}
          onKeyDown={(event) => event.stopPropagation()}
          role="presentation">
          {footerRight}
        </div>
      </div>
    </div>
  );
};

export const IntegrationAddCard = ({
  label,
  disabled = false,
  onClick,
}: {
  label: string;
  disabled?: boolean;
  onClick: () => void;
}) => (
  <button
    type="button"
    onClick={disabled ? undefined : onClick}
    disabled={disabled}
    className={cn(
      'flex h-35 flex-col items-center justify-center gap-3 rounded-lg border border-dashed p-(--surface-padding) text-muted-foreground hover:text-foreground',
      disabled && 'hover:cursor-not-allowed opacity-70',
    )}>
    <Plus className="h-5 w-5" />
    <span className="text-sm font-medium">{label}</span>
  </button>
);

const getLookupResourceName = (type: LookupResourceType | undefined) => {
  if (!type) return 'Resources';
  return type in PluralResourceMap ? PluralResourceMap[type as keyof typeof PluralResourceMap] : 'Resources';
};

export function ResourceSelectorField<T extends { id: string; name: string }>({
  id,
  'aria-label': ariaLabel,
  sourceType,
  targetType,
  selected,
  onSelect,
  items: providedItems,
  renderItem,
  disabled,
  align = 'start',
  placeholder,
  searchPlaceholder,
  className,
  sourceResourceId,
  platformId,
  queryEnabled = true,
  allowClear = true,
  clearLabel = 'None',
  emptyMessage = 'No results found.',
  isItemDisabled,
}: {
  id?: string;
  'aria-label'?: string;
  sourceType?: LookupResourceType;
  selected?: T | string | undefined;
  onSelect?: (item: T | undefined) => void;
  renderItem?: (item: T) => ReactNode;
  disabled?: boolean;
  align?: 'start' | 'center' | 'end';
  placeholder?: string;
  searchPlaceholder?: string;
  className?: string;
  sourceResourceId?: string;
  platformId?: string;
  queryEnabled?: boolean;
  allowClear?: boolean;
  clearLabel?: string;
  emptyMessage?: ReactNode;
  isItemDisabled?: (item: T) => boolean;
} & ({ targetType: LookupResourceType; items?: T[] } | { targetType?: never; items: T[] })) {
  const accessibility = useFormFieldAccessibility();
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');
  const { ref: triggerRef, width: contentWidth, measure } = useMeasuredWidth(400);

  const read = useRead(
    `lookup`,
    targetType
      ? {
          query: {
            TargetResourceType: targetType,
            SourceResourceType: sourceType,
            SourceResourceId: sourceResourceId,
            PlatformId: platformId,
          },
        }
      : undefined,
    { enabled: queryEnabled && targetType !== undefined && providedItems === undefined },
  );
  const lookupItems = useMemo<T[]>(() => {
    const lookupData = read.data?.data as unknown;
    if (Array.isArray(lookupData)) {
      return lookupData as T[];
    }

    if (lookupData && typeof lookupData === 'object') {
      const firstValue = Object.values(lookupData as Record<string, unknown>).at(0);
      if (Array.isArray(firstValue)) {
        return firstValue as T[];
      }
    }

    return [];
  }, [read.data?.data]);
  const items = providedItems ?? lookupItems;
  const selectedItem =
    typeof selected === 'string'
      ? items.find((i) => i.id === selected || (i as unknown as { dockerImageId?: string }).dockerImageId === selected)
      : selected;

  const filtered = filterBySplit(items, search, (i) => i.name).sort((a, b) => a.name.localeCompare(b.name));
  const grouped = useMemo(() => {
    const entries = new Map<string, T[]>();
    for (const item of filtered) {
      const group = (item as T & { group?: string | null }).group ?? '';
      const current = entries.get(group);
      if (current) current.push(item);
      else entries.set(group, [item]);
    }
    return [...entries.entries()];
  }, [filtered]);

  const handleSelect = (item: T | undefined) => {
    onSelect?.(item);
    setOpen(false);
  };

  useLayoutEffect(() => {
    if (open) measure();
  }, [open, selectedItem, measure]);

  const defaultDisplay = (item: T | undefined) => (item?.name == '' ? `<id=${item?.id}>` : item?.name);
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          id={id}
          ref={triggerRef as any}
          disabled={disabled}
          variant="ghost"
          role="combobox"
          aria-expanded={open}
          aria-label={ariaLabel ?? accessibility?.label}
          aria-describedby={accessibility?.describedBy}
          aria-invalid={accessibility?.invalid || undefined}
          data-placeholder={selectedItem ? undefined : ''}
          className={cn(
            'flex justify-between gap-2 w-full max-w-100 font-normal data-placeholder:text-muted-foreground text-sm bg-background hover:bg-background shadow-xs border',
            className,
          )}>
          <span className="min-w-0 flex-1 truncate text-left" title={defaultDisplay(selectedItem)}>
            {defaultDisplay(selectedItem) ?? placeholder}
          </span>
          <ChevronDown className="h-4 w-4 shrink-0 opacity-60" />
        </Button>
      </PopoverTrigger>

      <PopoverContent
        align={align}
        className="w-full max-w-100 p-0 bg-background"
        style={contentWidth ? { width: `${contentWidth}px` } : undefined}>
        <Command shouldFilter={false} defaultValue={selectedItem?.name ?? '__none__'}>
          <CommandInput
            placeholder={searchPlaceholder ?? `Search ${getLookupResourceName(sourceType ?? targetType)}`}
            value={search}
            onValueChange={setSearch}
          />

          <CommandList>
            <CommandEmpty>{emptyMessage}</CommandEmpty>

            <CommandGroup>
              {allowClear && !search && (
                <CommandItem
                  value="__none__"
                  onSelect={() => handleSelect(undefined)}
                  className="flex items-center justify-between cursor-pointer my-0.5 px-2 py-2 rounded-sm">
                  <span>{clearLabel}</span>
                  <Check className={cn('h-4 w-4 transition-opacity', !selectedItem ? 'opacity-100' : 'opacity-0')} />
                </CommandItem>
              )}
            </CommandGroup>
            {grouped.map(([group, groupItems]) => (
              <CommandGroup key={group || '__ungrouped__'} heading={group || undefined}>
                {groupItems.map((item) => {
                  const isSelected = selectedItem?.id === item.id;
                  return (
                    <CommandItem
                      key={item.id}
                      disabled={isItemDisabled?.(item)}
                      value={defaultDisplay(item)}
                      onSelect={() => handleSelect(item)}
                      className="flex items-start justify-between cursor-pointer my-0.5 px-2 py-2 rounded-sm">
                      <div className="min-w-0 flex-1">{renderItem?.(item) ?? <span>{defaultDisplay(item)}</span>}</div>
                      <Check className={cn('h-4 w-4 transition-opacity', isSelected ? 'opacity-100' : 'opacity-0')} />
                    </CommandItem>
                  );
                })}
              </CommandGroup>
            ))}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}

export function MultiResourceSelectorField<T extends { id: string; name: string }>({
  sourceType,
  targetType,
  selected,
  onSelect,
  items: providedItems,
  disabled,
  placeholder,
  platformId,
  className,
  valueKey = 'id',
  sourceResourceId,
  queryEnabled = true,
  disableUnselectedOptions = false,
}: {
  sourceType: LookupResourceType;
  targetType: LookupResourceType;
  selected?: string[] | T[];
  onSelect?: (items: T[]) => void;
  items?: T[];
  disabled?: boolean;
  align?: 'start' | 'center' | 'end';
  placeholder?: string;
  platformId?: string;
  className?: string;
  valueKey?: 'id' | 'name';
  sourceResourceId?: string;
  queryEnabled?: boolean;
  disableUnselectedOptions?: boolean;
}) {
  const resourceName = getLookupResourceName(sourceType);
  const read = useRead(
    `lookup`,
    {
      query: {
        TargetResourceType: targetType,
        SourceResourceType: sourceType,
        SourceResourceId: sourceResourceId,
        PlatformId: platformId,
      },
    },
    { enabled: queryEnabled && providedItems === undefined },
  );

  const lookupItems = useMemo<T[]>(() => {
    const lookupData = read.data?.data as unknown;
    if (Array.isArray(lookupData)) return lookupData as T[];

    if (lookupData && typeof lookupData === 'object') {
      const firstValue = Object.values(lookupData as Record<string, unknown>).at(0);
      if (Array.isArray(firstValue)) return firstValue as T[];
    }

    return [];
  }, [read.data?.data]);
  const items = providedItems ?? lookupItems;

  const getValue = useCallback((item: T) => (valueKey === 'name' ? item.name : item.id), [valueKey]);

  const selectedIds = useMemo(() => {
    if (!selected) return [];
    return selected.map((s) => {
      if (typeof s !== 'string') return getValue(s);
      if (valueKey === 'name') {
        const match = items.find((item) => item.id === s || item.name === s);
        return match ? match.name : s;
      }
      return s;
    });
  }, [selected, getValue, items, valueKey]);

  const options: MultiSelectOption[] = useMemo(
    () =>
      items
        .map((item) => {
          const value = getValue(item);
          return {
            label: item.name,
            value,
            disabled: disableUnselectedOptions && !selectedIds.includes(value),
          };
        })
        .sort((a, b) => a.label.localeCompare(b.label)),
    [disableUnselectedOptions, getValue, items, selectedIds],
  );

  const handleValueChange = (newIds: string[]) => {
    const newSelectedItems = items.filter((item) => newIds.includes(getValue(item)));
    onSelect?.(newSelectedItems);
  };

  return (
    <MultiSelect
      options={options}
      defaultValue={selectedIds}
      onValueChange={handleValueChange}
      placeholder={
        read.isLoading && providedItems === undefined ? 'Loading...' : (placeholder ?? `Select ${resourceName}...`)
      }
      disabled={disabled || (read.isLoading && providedItems === undefined)}
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

export const extractIds = (value: unknown): string[] => {
  if (!Array.isArray(value)) return [];

  return value.flatMap((item) => {
    if (typeof item === 'string') {
      return item;
    }

    if (item && typeof item === 'object' && 'id' in item) {
      const id = (item as { id?: unknown }).id;
      if (typeof id === 'string') {
        return id;
      }
    }

    return [];
  });
};

type IdSearchMultiSelectOption = {
  label: string;
  value: string;
};

export const IdSearchMultiSelectField = ({
  value,
  onChange,
  options,
  isLoading,
  searchValue,
  onSearchValueChange,
  loadingPlaceholder,
  selectPlaceholder,
  searchingEmptyIndicator,
  emptyIndicator,
}: {
  value: string[] | null;
  onChange: (ids: string[]) => void;
  options: IdSearchMultiSelectOption[];
  isLoading: boolean;
  searchValue: string;
  onSearchValueChange: (value: string) => void;
  loadingPlaceholder: string;
  selectPlaceholder: string;
  searchingEmptyIndicator: string;
  emptyIndicator: string;
}) => {
  return (
    <div className="max-w-100">
      <MultiSelect
        options={options}
        defaultValue={(value ?? []).map(String)}
        onValueChange={onChange}
        placeholder={isLoading ? loadingPlaceholder : selectPlaceholder}
        searchable
        searchValue={searchValue}
        onSearchValueChange={onSearchValueChange}
        disableLocalSearchFilter
        emptyIndicator={isLoading ? searchingEmptyIndicator : emptyIndicator}
        maxCount={5}
        animation={0}
        resetOnDefaultValueChange={true}
      />
    </div>
  );
};

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
          <span title={value} className="font-medium text-nowrap max-w-50 overflow-hidden text-ellipsis">
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

export const DockerContainerCell = ({
  name,
  id,
  state,
  platformId,
}: {
  name: string;
  id: string;
  state: string;
  platformId: string;
}) => (
  <div className="flex flex-wrap gap-1 items-center">
    <StateIndicator value={state ?? ContainerStateStatus.Exited} kind="container" />
    <Link to={`/platforms/${platformId}/containers/${normalizeDockerId(id)}`} className="table-link" title={name}>
      {truncate(name?.replace(/^\//, ''), 24)}
    </Link>
  </div>
);

export const DockerImageCell = ({ children }: { children: React.ReactNode }) => (
  <div className="flex flex-wrap gap-2 items-center text-foreground">
    <HardDrive width={13} height={13} className="text-primary" />
    {children}
  </div>
);

export const DockerNetworksCell = ({
  networks,
  platformId,
}: {
  networks?: Record<string, string>;
  platformId?: string;
}) => {
  if (!networks || Object.keys(networks).length === 0) return null;

  return (
    <div className="text-foreground gap-2 flex flex-wrap items-center">
      <Network width={13} height={13} className="text-primary" />
      {Object.entries(networks).map(([name, id]) => (
        <Link to={`/platforms/${platformId}/networks/${formatId(id)}`} key={id} title={name} className="table-link">
          {truncate(name, 24)}
        </Link>
      ))}
    </div>
  );
};

export const DockerVolumesCell = ({ volumes, platformId }: { volumes?: string[]; platformId?: string }) => {
  if (!volumes || volumes.length === 0) return null;

  return (
    <div className="text-foreground gap-2 flex flex-wrap items-center">
      <Database width={13} height={13} className="text-primary" />
      {volumes.map((volume) => (
        <Link to={`/platforms/${platformId}/volumes/${volume}`} title={volume} key={volume} className="table-link">
          {truncate(volume, 12)}
        </Link>
      ))}
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
    className: 'text-success',
  },
  [UpdateBehavior.Notify]: {
    label: 'Notify',
    Icon: Bell,
    className: 'text-warning',
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
    className: 'text-success',
  },
  [AutoUpdateStatus.Failed]: {
    label: 'Failed',
    Icon: CircleX,
    className: 'text-danger',
  },
  [AutoUpdateStatus.UpdateAvailable]: {
    label: 'Update Available',
    Icon: Info,
    className: 'text-warning',
  },
  [AutoUpdateStatus.Updating]: {
    label: 'Updating...',
    Icon: RefreshCcwDot,
    className: 'text-pending',
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

export const UpdateAvailableNotice = ({
  title = 'Update available:',
  actionLabel,
  targetLabel,
  sourceLabel,
  sourceTitle,
  currentLabel,
  nextLabel,
  currentTitle,
  nextTitle,
  dismissible = true,
  actions,
}: {
  title?: string;
  actionLabel: string;
  targetLabel: string;
  sourceLabel?: ReactNode;
  sourceTitle?: string | null;
  currentLabel?: ReactNode;
  nextLabel?: ReactNode;
  currentTitle?: string | null;
  nextTitle?: string | null;
  dismissible?: boolean;
  actions?: ReactNode;
}) => {
  const [dismissed, setDismissed] = useState(false);

  if (dismissible && dismissed) return null;

  return (
    <div className="flex items-center justify-between gap-4 rounded-md border border-amber-200 bg-amber-50/50 px-4 py-3">
      <div className="flex items-center gap-3 overflow-hidden">
        <ArrowUpCircle className="h-4 w-4 shrink-0 text-amber-500" />
        <div className="flex items-center gap-2 truncate text-sm text-muted-foreground">
          <span className="font-mono text-foreground/80">{title}</span>
          <span className="truncate">
            Click <span className="font-mono text-foreground/80">{actionLabel}</span> to apply the update to this{' '}
            {targetLabel}.
          </span>
          {sourceLabel ? (
            <span className="truncate text-xs text-muted-foreground" title={sourceTitle ?? undefined}>
              {sourceLabel}
            </span>
          ) : null}
          {currentLabel || nextLabel ? (
            <div className="flex shrink-0 items-center gap-1.5">
              {currentLabel ? (
                <span className="text-xs text-muted-foreground line-through" title={currentTitle ?? undefined}>
                  {currentLabel}
                </span>
              ) : null}
              {currentLabel && nextLabel ? <ArrowRight className="h-3 w-3 text-muted-foreground" /> : null}
              {nextLabel ? (
                <span className="text-xs font-medium text-amber-700" title={nextTitle ?? undefined}>
                  {nextLabel}
                </span>
              ) : null}
            </div>
          ) : null}
        </div>
      </div>

      <div className="flex shrink-0 items-center gap-2">
        {actions}
        {dismissible ? (
          <button
            onClick={() => setDismissed(true)}
            className="shrink-0 rounded-md p-1 text-slate-400 transition-colors hover:bg-amber-100/50 hover:text-slate-600"
            aria-label="Dismiss">
            <X className="h-4 w-4" />
          </button>
        ) : null}
      </div>
    </div>
  );
};

export const MemoryUsageCell = ({
  state,
  stats,
  className,
}: {
  state: ContainerStateStatus;
  stats?: Partial<ContainerStatView> | null;
  className?: string;
}) => {
  if (state !== ContainerStateStatus.Running) {
    return <div className={cn('text-muted-foreground', className)}>0B / 0B</div>;
  }
  return (
    <span className={cn('text-[13px]', className)} title="Memory usage excluding file cache / memory limit">
      {byteTransform(stats?.memoryActive ?? 0, 2) + ' / ' + byteTransform(stats?.memoryLimit ?? 0, 2)}
    </span>
  );
};

export const CPUCell = ({
  state,
  stats,
  className,
}: {
  state: ContainerStateStatus;
  stats?: Partial<ContainerStatView> | null;
  className?: string;
}) => {
  if (state !== ContainerStateStatus.Running) {
    return <div className={cn('text-muted-foreground', className)}>0%</div>;
  }
  return (
    <span className={cn('text-[13px]', className)}>
      {stats?.cpuUsage ? toFixedNumber(stats?.cpuUsage as number, 'percent') : '0%'}
    </span>
  );
};

export const PlatformStatusCell = ({
  status,
  id,
  name,
}: {
  status: PlatformStatus;
  id: string;
  name: string | undefined;
}) => {
  return (
    <div className="flex flex-row items-center gap-2">
      <Server width={13} height={13} className={status === PlatformStatus.Online ? 'text-success' : 'text-danger'} />
      <Link to={`/platforms/edit/${id}`} className="table-link" title={name}>
        {name}
      </Link>
    </div>
  );
};

export const ActorCell = ({ type, name }: { type: ActorType; name: string | undefined }) => {
  return (
    <div className="flex flex-row items-center gap-2">
      {type === ActorType.User && <User width={13} height={13} className="text-foreground/80" />}
      {type === ActorType.System && <Settings width={13} height={13} className="text-foreground/80" />}
      {name}
    </div>
  );
};

export const TargetCell = ({
  resourceType,
  resourceId,
  resourceName,
  dockerNodeId,
}: {
  resourceType: ActivityResourceType | AlertResourceType;
  resourceId: string | undefined;
  resourceName: string | undefined;
  dockerNodeId?: string | null;
}) => {
  const resourceConfig: Partial<Record<string, { Icon: any; path: string }>> = {
    [ActivityResourceType.AlertRule]: { Icon: Megaphone, path: `/alert-rules/edit/${resourceId}` },
    [ActivityResourceType.Deployment]: { Icon: Rocket, path: `/deployments/edit/${resourceId}` },
    [ActivityResourceType.Registry]: { Icon: Cable, path: `/registries/edit/${resourceId}` },
    [ActivityResourceType.Platform]: { Icon: Server, path: `/platforms/edit/${resourceId}` },
    [ActivityResourceType.Stack]: { Icon: Layers, path: `/stacks/edit/${resourceId}` },
    [ActivityResourceType.SwarmService]: { Icon: Boxes, path: `/swarm-services/edit/${resourceId}` },
    [ActivityResourceType.GitRepository]: { Icon: GitBranch, path: `/git-repos/edit/${resourceId}` },
    [ActivityResourceType.Script]: { Icon: Workflow, path: `/scripts/edit/${resourceId}` },
    [ActivityResourceType.Build]: { Icon: Hammer, path: `/builds/edit/${resourceId}` },
    [ActivityResourceType.BuildAgentPool]: { Icon: ServerPlus, path: `/build-pools/edit/${resourceId}` },
    [ActivityResourceType.BackupPolicy]: { Icon: CalendarClock, path: `/backup-policies/edit/${resourceId}` },
    [ActivityResourceType.User]: { Icon: User, path: `/access/users/edit/${resourceId}` },
    [ActivityResourceType.Team]: { Icon: Users, path: `/access/teams/edit/${resourceId}` },
    [ActivityResourceType.Role]: { Icon: Shield, path: '/access/roles' },
    [ActivityResourceType.ServiceAccount]: {
      Icon: Bot,
      path: `/access/service-accounts/edit/${resourceId}`,
    },
    [ActivityResourceType.Volume]: {
      Icon: Database,
      path: resourceName
        ? `/platforms/${resourceId}/volumes/${encodeURIComponent(resourceName)}${dockerNodeId ? `?dockerNodeId=${encodeURIComponent(dockerNodeId)}` : ''}`
        : `/platforms/${resourceId}/volumes`,
    },
  };

  const config = resourceConfig[resourceType];
  if (!config) return null;
  const { Icon, path } = config;

  return (
    <div className="flex flex-row items-center gap-2">
      <Icon width={13} height={13} className="text-foreground/80" />
      <Link to={path} className="table-link">
        {resourceName}
      </Link>
    </div>
  );
};

type PaginationControlsProps = {
  currentPage: number;
  totalPages: number;
  onPageChange: (page: number) => void;
  className?: string;
};

export const PaginationControls = ({ currentPage, totalPages, onPageChange, className }: PaginationControlsProps) => {
  const pageItems = useMemo(() => buildPageItems(currentPage, totalPages), [currentPage, totalPages]);

  if (totalPages <= 1) return null;

  return (
    <Pagination className={className}>
      <PaginationContent>
        <PaginationItem>
          <PaginationPrevious
            size={'sm'}
            href="#"
            onClick={(event) => {
              event.preventDefault();
              onPageChange(currentPage - 1);
            }}
            aria-disabled={currentPage <= 1}
            className={currentPage <= 1 ? 'pointer-events-none opacity-50' : undefined}
          />
        </PaginationItem>
        {pageItems.map((item, index) =>
          item === 'ellipsis' ? (
            <PaginationItem key={`ellipsis-${index}`}>
              <PaginationEllipsis />
            </PaginationItem>
          ) : (
            <PaginationItem key={item}>
              <PaginationLink
                size={'sm'}
                href="#"
                isActive={item === currentPage}
                onClick={(event) => {
                  event.preventDefault();
                  onPageChange(item);
                }}>
                {item}
              </PaginationLink>
            </PaginationItem>
          ),
        )}
        <PaginationItem>
          <PaginationNext
            size={'sm'}
            href="#"
            onClick={(event) => {
              event.preventDefault();
              onPageChange(currentPage + 1);
            }}
            aria-disabled={currentPage >= totalPages}
            className={currentPage >= totalPages ? 'pointer-events-none opacity-50' : undefined}
          />
        </PaginationItem>
      </PaginationContent>
    </Pagination>
  );
};

const buildPageItems = (currentPage: number, totalPages: number) => {
  const pages = new Set<number>();
  pages.add(1);
  pages.add(totalPages);
  pages.add(currentPage);
  pages.add(currentPage - 1);
  pages.add(currentPage + 1);

  const sorted = Array.from(pages)
    .filter((page) => page >= 1 && page <= totalPages)
    .sort((a, b) => a - b);

  const items: Array<number | 'ellipsis'> = [];
  for (let i = 0; i < sorted.length; i += 1) {
    const page = sorted[i];
    const prev = sorted[i - 1];
    if (prev && page - prev > 1) items.push('ellipsis');
    items.push(page);
  }

  return items;
};

export const filterBarClassName =
  'grid w-full grid-cols-1 gap-2 sm:ml-auto sm:w-fit sm:grid-cols-2 sm:justify-items-end xl:flex xl:flex-wrap xl:items-center xl:justify-end xl:gap-4';

export const filterFieldClassName = 'w-full min-w-0 sm:w-[220px]';

export const FilterBar = ({ children }: { children: React.ReactNode }) => {
  return <div className={filterBarClassName}>{children}</div>;
};

type PagedDataTableProps<TData extends { id?: string | null }> = {
  columns: ColumnDef<StockFeatures, TData>[];
  data: TData[];
  isLoading: boolean;
  query: { page: number; pageSize: number };
  setQuery: (patch: Partial<{ page: number; pageSize: number }>) => void;
  totalCount?: number | string | null;
  showPagination?: boolean;
  onSelectionChange?: (selectedRows: TData[]) => void;
  getRowId?: (row: TData) => string;
  enableRowSelection?: (row: TData) => boolean;
};

export function PagedDataTable<TData extends { id?: string | null }>({
  columns,
  data,
  isLoading,
  query,
  setQuery,
  totalCount,
  showPagination = true,
  onSelectionChange,
  getRowId,
  enableRowSelection,
}: PagedDataTableProps<TData>) {
  const tableTopRef = useRef<HTMLDivElement | null>(null);
  const totalPages = Math.max(1, Math.ceil(Number(totalCount ?? 0) / query.pageSize));

  const goToPage = (page: number) => {
    if (page < 1 || page > totalPages || page === query.page) return;
    setQuery({ page });
    requestAnimationFrame(() => {
      tableTopRef.current?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    });
  };

  const handlePageSizeChange = (value: string) => {
    setQuery({
      pageSize: Number(value),
    });
  };

  return (
    <div className="flex flex-col gap-4" ref={tableTopRef}>
      <DataTable
        columns={columns}
        data={data}
        isLoading={isLoading}
        onSelectionChange={onSelectionChange}
        getRowId={getRowId}
        enableRowSelection={enableRowSelection}
      />
      {showPagination && (
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <PaginationControls
            currentPage={query.page}
            totalPages={totalPages}
            onPageChange={goToPage}
            className="justify-start"
          />
          {totalPages > 1 && (
            <SelectField
              value={query.pageSize.toString()}
              options={pageSizeOptions}
              onChange={handlePageSizeChange}
              placeholder="Page Size"
              allLabel="Page Size"
              selectableLabel={false}
            />
          )}
        </div>
      )}
    </div>
  );
}

export function SelectField({
  value,
  onChange,
  options,
  placeholder,
  allLabel,
  allIcon: AllIcon,
  selectableLabel = true,
  className,
  disabled,
}: {
  value: string;
  onChange: (value: string) => void;
  options: (string | { value: string; label: string; icon?: React.ComponentType<{ className?: string }> })[];
  placeholder: string;
  allLabel: string;
  allIcon?: React.ComponentType<{ className?: string }>;
  selectableLabel?: boolean;
  className?: string;
  disabled?: boolean;
}) {
  return (
    <div className={cn('min-w-50', className)}>
      <Select value={value} onValueChange={onChange} disabled={disabled}>
        <SelectTrigger className="w-full bg-background">
          <SelectValue placeholder={placeholder} />
        </SelectTrigger>
        <SelectContent className="bg-background">
          {selectableLabel ? (
            <>
              <SelectItem value="All">
                <div className="flex items-center gap-3">
                  {AllIcon && <AllIcon className="size-3.5 text-muted-foreground" />}
                  <span>{allLabel}</span>
                </div>
              </SelectItem>
              <SelectSeparator />
            </>
          ) : (
            <span className="p-2 text-muted-foreground text-sm">{allLabel}</span>
          )}
          {options.map((option) => {
            if (typeof option === 'string') {
              return (
                <SelectItem key={option} value={option}>
                  {option}
                </SelectItem>
              );
            }
            const Icon = option.icon;
            return (
              <SelectItem key={option.value} value={option.value}>
                <div className="flex items-center gap-3">
                  {Icon && <Icon className="size-3.5 text-muted-foreground" />}
                  <span>{option.label}</span>
                </div>
              </SelectItem>
            );
          })}
        </SelectContent>
      </Select>
    </div>
  );
}

export function UnmanagedResourceIcon({ title }: { title: string }) {
  return (
    <span className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-sm text-amber-500" title={title}>
      <Unlink className="h-3 w-3" aria-label={title} />
    </span>
  );
}

export const pageSizeOptions = [
  { value: '25', label: '25' },
  { value: '50', label: '50' },
  { value: '100', label: '100' },
  { value: '200', label: '200' },
];
