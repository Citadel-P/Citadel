import { useMemo, useState, useLayoutEffect, useRef, useCallback, useDeferredValue, memo, type ReactNode } from 'react';
import { ColumnDef } from '@tanstack/react-table';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import {
  ActorType,
  ActivityStatus,
  AutoUpdateStatus,
  ContainerStateStatus,
  ContainerStatView,
  LookupResourceType,
  PlatformStatus,
  UpdateBehavior,
  ActivityResourceType,
  AlertSeverity,
  AlertResourceType,
  AlertEventStatus,
} from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Command, CommandInput, CommandList, CommandGroup, CommandItem, CommandEmpty } from '@/components/ui/command';
import { DataTable } from '@/components/ui/data-table';
import {
  Cable,
  Check,
  ChevronDown,
  GitBranch,
  Info,
  Layers,
  LucideIcon,
  Megaphone,
  Rocket,
  Server,
  Settings,
  Tags,
  User,
} from 'lucide-react';
import { cn, filterBySplit, normalizeDockerId, toFixedNumber } from '@/lib/utils';
import { PluralResourceMap } from '@/api/types';
import { useMeasuredWidth, useRead, useLocalStorage } from '@/lib/hooks';
import Convert from 'ansi-to-html';
import { Badge } from '../ui/badge';
import { CardDescription, CardHeader, CardTitle } from '../ui/card';
import { ContentCard } from './content-card';
import { MultiSelect, MultiSelectOption } from '../ui/multi-select';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import {
  Pagination,
  PaginationContent,
  PaginationEllipsis,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from '@/components/ui/pagination';
import {
  Bell,
  CircleCheck,
  CircleQuestionMark,
  CircleX,
  Database,
  Eraser,
  Funnel,
  HardDrive,
  Network,
  RefreshCcw,
  RefreshCcwDot,
  RefreshCwOff,
  Timer,
  WrapText,
} from 'lucide-react';
import { byteTransform } from '@/lib/bytes.helper';
import { Link } from 'react-router';
import { Select, SelectContent, SelectItem, SelectSeparator, SelectTrigger, SelectValue } from '@/components/ui/select';
import { truncate } from '@/lib/truncate';
import { formatId } from '@/lib/utils';
import { StateIndicator } from './state-indicator';
import { getContainerSeriesColor } from './container-series-colors';

export type StatsWindowHours = 24 | 48 | 72;

export const STATS_WINDOWS = [
  { value: 24, label: 'Last 24 hours' },
  { value: 48, label: 'Last 48 hours' },
  { value: 72, label: 'Last 72 hours' },
] as const satisfies readonly { value: StatsWindowHours; label: string }[];

export const StatsWindowSelect = ({
  value,
  onChange,
}: {
  value: StatsWindowHours;
  onChange: (hours: StatsWindowHours) => void;
}) => (
  <Select value={String(value)} onValueChange={(nextValue) => onChange(Number(nextValue) as StatsWindowHours)}>
    <SelectTrigger className="h-8 w-36 rounded-sm bg-background shadow-none">
      <SelectValue />
    </SelectTrigger>
    <SelectContent className="bg-background">
      {STATS_WINDOWS.map((option) => (
        <SelectItem key={option.value} value={String(option.value)}>
          {option.label}
        </SelectItem>
      ))}
    </SelectContent>
  </Select>
);

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

export function ResourceSelectorField<T extends { id: string; name: string }>({
  sourceType,
  targetType,
  selected,
  onSelect,
  disabled,
  align = 'start',
  placeholder,
  className,
  sourceResourceId,
  platformId,
  queryEnabled = true,
}: {
  sourceType: LookupResourceType;
  targetType: LookupResourceType;
  selected?: T | string | undefined;
  onSelect?: (item: T | undefined) => void;
  disabled?: boolean;
  align?: 'start' | 'center' | 'end';
  placeholder?: string;
  className?: string;
  sourceResourceId?: string;
  platformId?: string;
  queryEnabled?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');
  const { ref: triggerRef, width: contentWidth, measure } = useMeasuredWidth(400);

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
    { enabled: queryEnabled },
  );
  const items = useMemo<T[]>(() => {
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
  const selectedItem =
    typeof selected === 'string'
      ? items.find((i) => i.id === selected || (i as unknown as { dockerImageId?: string }).dockerImageId === selected)
      : selected;

  const filtered = filterBySplit(items, search, (i) => i.name).sort((a, b) => a.name.localeCompare(b.name));

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
          ref={triggerRef as any}
          disabled={disabled}
          variant="ghost"
          role="combobox"
          aria-expanded={open}
          data-placeholder={selectedItem ? undefined : ''}
          className={cn(
            'flex justify-between gap-2 w-full max-w-100 font-normal data-placeholder:text-muted-foreground text-sm bg-background hover:bg-background shadow-xs border',
            className,
          )}>
          {defaultDisplay(selectedItem) ?? placeholder}
          <ChevronDown className="h-4 w-4 opacity-60" />
        </Button>
      </PopoverTrigger>

      <PopoverContent
        align={align}
        className="w-full max-w-100 p-0 bg-background"
        style={contentWidth ? { width: `${contentWidth}px` } : undefined}>
        <Command shouldFilter={false} defaultValue={selectedItem?.name ?? '__none__'}>
          <CommandInput
            placeholder={`Search ${PluralResourceMap[sourceType]}`}
            value={search}
            onValueChange={setSearch}
          />

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
                    value={defaultDisplay(item)}
                    onSelect={() => handleSelect(item)}
                    className="flex items-center justify-between cursor-pointer my-0.5 px-2 py-2 rounded-sm">
                    <span>{defaultDisplay(item)}</span>
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
  sourceType,
  targetType,
  selected,
  onSelect,
  disabled,
  placeholder,
  platformId,
  className,
  valueKey = 'id',
  sourceResourceId,
  queryEnabled = true,
}: {
  sourceType: LookupResourceType;
  targetType: LookupResourceType;
  selected?: string[] | T[];
  onSelect?: (items: T[]) => void;
  disabled?: boolean;
  align?: 'start' | 'center' | 'end';
  placeholder?: string;
  platformId?: string;
  className?: string;
  valueKey?: 'id' | 'name';
  sourceResourceId?: string;
  queryEnabled?: boolean;
}) {
  const resourceName = PluralResourceMap[sourceType];
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
    { enabled: queryEnabled },
  );

  const items = useMemo<T[]>(() => {
    const lookupData = read.data?.data as unknown;
    if (Array.isArray(lookupData)) return lookupData as T[];

    if (lookupData && typeof lookupData === 'object') {
      const firstValue = Object.values(lookupData as Record<string, unknown>).at(0);
      if (Array.isArray(firstValue)) return firstValue as T[];
    }

    return [];
  }, [read.data?.data]);

  const getValue = useCallback((item: T) => (valueKey === 'name' ? item.name : item.id), [valueKey]);

  const options: MultiSelectOption[] = useMemo(
    () =>
      items
        .map((item) => ({
          label: item.name,
          value: getValue(item),
        }))
        .sort((a, b) => a.label.localeCompare(b.label)),
    [items, getValue],
  );

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

  const handleValueChange = (newIds: string[]) => {
    const newSelectedItems = items.filter((item) => newIds.includes(getValue(item)));
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
  state: ContainerStateStatus;
  platformId: string;
}) => (
  <div className="flex flex-wrap gap-1 items-center">
    <StateIndicator value={state ?? ContainerStateStatus.Exited} />
    <Link to={`/platforms/${platformId}/containers/${normalizeDockerId(id)}`} className="table-link" title={name}>
      {truncate(name?.slice(1), 24)}
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
    Icon: Info,
    className: 'text-orange-500',
  },
  [AutoUpdateStatus.Updating]: {
    label: 'Updating...',
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

export interface LogEntry {
  timestamp?: string;
  message: string;
}

interface RenderedLogEntry extends LogEntry {
  html: string;
  containerName?: string;
}

interface LogViewerProps {
  logs: string | string[] | LogEntry[];
  autoScroll?: boolean;
  className?: string;
  showTimestamps?: boolean;
  wrapLines?: boolean;
  timeStamps?: boolean;
  allowWrap?: boolean;
  onClear?: () => void;
  containerFilters?: string[];
  enableContainerFilter?: boolean;
}

interface QuickActionProps {
  label: string;
  icon: React.ReactNode;
  active?: boolean;
  disabled?: boolean;
  side?: 'left' | 'top' | 'right' | 'bottom';
  onClick: () => void;
}

export const QuickAction = ({ label, icon, active, disabled, side = 'left', onClick }: QuickActionProps) => (
  <TooltipProvider delayDuration={200}>
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          disabled={disabled}
          size="icon-sm"
          variant={active ? 'secondary' : 'outline'}
          className={'rounded-full h-7 w-7 shadow-sm'}
          onClick={onClick}>
          {icon}
        </Button>
      </TooltipTrigger>
      <TooltipContent side={side}>{label}</TooltipContent>
    </Tooltip>
  </TooltipProvider>
);

const ansiConverter = new Convert({
  newline: false,
  escapeXML: true,
  stream: true,
});

// Memoised row so React skips it when only scroll position changes.
const LogRow = memo(
  ({ log, showTimestamps, wrapLines }: { log: RenderedLogEntry; showTimestamps: boolean; wrapLines: boolean }) => {
    const containerColor = log.containerName ? getContainerSeriesColor(log.containerName) : undefined;

    return (
      <div className={cn('flex gap-2 min-h-[1.2rem]', wrapLines ? 'whitespace-pre-wrap break-all' : 'whitespace-pre')}>
        {showTimestamps && log.timestamp && (
          <span className="text-foreground/30 shrink-0 select-none tabular-nums">
            {log.timestamp.includes('T') ? log.timestamp.split('T')[1].slice(0, 8) : log.timestamp}
          </span>
        )}
        {log.containerName && (
          <span className={cn('shrink-0 select-none font-semibold', containerColor?.text)}>[{log.containerName}]</span>
        )}
        <span dangerouslySetInnerHTML={{ __html: log.html }} />
      </div>
    );
  },
);
LogRow.displayName = 'LogRow';

const parseLogContainerLabel = (message: string): { containerName?: string; message: string } => {
  const match = message.match(/^\[([^\]]+)\]\s?(.*)$/s);
  if (!match) return { message };

  return {
    containerName: match[1],
    message: match[2] ?? '',
  };
};

function LogContainerFilter({
  containers,
  selectedContainers,
  onToggle,
}: {
  containers: string[];
  selectedContainers: string[];
  onToggle: (container: string) => void;
}) {
  const selectedSet = useMemo(() => new Set(selectedContainers), [selectedContainers]);
  const active = selectedContainers.length !== containers.length;

  return (
    <Popover>
      <TooltipProvider delayDuration={200}>
        <Tooltip>
          <TooltipTrigger asChild>
            <PopoverTrigger asChild>
              <Button
                size="icon-sm"
                variant={active ? 'secondary' : 'outline'}
                className="h-7 w-7 rounded-full shadow-sm">
                <Funnel className="h-3.5 w-3.5" />
              </Button>
            </PopoverTrigger>
          </TooltipTrigger>
          <TooltipContent side="left">Container filter</TooltipContent>
        </Tooltip>
      </TooltipProvider>
      <PopoverContent align="end" side="left" className="w-64 p-2 bg-background">
        <div className="px-2 pb-2 text-xs font-medium text-muted-foreground">Containers</div>
        <div className="max-h-64 overflow-auto">
          {containers.map((container) => {
            const color = getContainerSeriesColor(container);
            const selected = selectedSet.has(container);
            return (
              <button
                type="button"
                key={container}
                className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent"
                onClick={() => onToggle(container)}>
                <span
                  className={cn(
                    'flex size-4 shrink-0 items-center justify-center border',
                    selected && 'bg-primary text-primary-foreground',
                  )}>
                  {selected && <Check className="size-3" />}
                </span>
                <span className={cn('h-2 w-2 shrink-0 rounded-full', color.dot)} />
                <span className="truncate" title={container}>
                  {container}
                </span>
              </button>
            );
          })}
        </div>
      </PopoverContent>
    </Popover>
  );
}

// Simple virtual-list constants.
const ITEM_HEIGHT = 20; // px — matches min-h-[1.2rem] + text-xs
const VIEWPORT_HEIGHT = 600; // px — matches max-h-[600px]
const OVERSCAN = 10;

export const LogViewer = memo(
  ({
    logs,
    autoScroll = true,
    className,
    showTimestamps: initialShowTimestamps = false,
    wrapLines: initialWrapLines = true,
    timeStamps: enableTimestamps = false,
    allowWrap = false,
    onClear,
    containerFilters,
    enableContainerFilter,
  }: LogViewerProps) => {
    const scrollRef = useRef<HTMLDivElement>(null);
    const [isAtBottom, setIsAtBottom] = useState(true);
    const [showTimestamps, setShowTimestamps] = useLocalStorage('log-viewer-show-timestamps', initialShowTimestamps);
    const [wrapLines, setWrapLines] = useLocalStorage('log-viewer-wrap-lines', initialWrapLines);
    const [excludedContainers, setExcludedContainers] = useState<string[]>([]);
    const [scrollTop, setScrollTop] = useState(0);

    // Defer heavy log updates so toggles / scroll stay responsive.
    const deferredLogs = useDeferredValue(logs);
    const containerFilteringEnabled = enableContainerFilter ?? Boolean(containerFilters?.length);

    const normalizedLogs = useMemo((): LogEntry[] => {
      if (!deferredLogs) return [];

      if (typeof deferredLogs === 'string') {
        return parseAnsiTerminalStream(deferredLogs).map((line) => ({ message: line }));
      }

      return deferredLogs.flatMap((log) => {
        if (typeof log === 'object' && log !== null) {
          return parseAnsiTerminalStream(log.message).map((line) => ({
            timestamp: log.timestamp,
            message: line,
          }));
        }

        return parseAnsiTerminalStream(String(log)).map((line) => ({ message: line }));
      });
    }, [deferredLogs]);

    const availableContainers = useMemo(() => {
      if (!containerFilteringEnabled) return [];

      const fromProps = (containerFilters ?? []).map((container) => container.trim()).filter(Boolean);
      if (fromProps.length > 0) {
        return Array.from(new Set(fromProps)).sort((a, b) => a.localeCompare(b));
      }

      const fromLogs = normalizedLogs
        .map((log) => parseLogContainerLabel(log.message).containerName)
        .filter((container): container is string => Boolean(container));

      return Array.from(new Set(fromLogs)).sort((a, b) => a.localeCompare(b));
    }, [containerFilteringEnabled, containerFilters, normalizedLogs]);

    const selectedContainers = useMemo(() => {
      const excluded = new Set(excludedContainers);
      return availableContainers.filter((container) => !excluded.has(container));
    }, [availableContainers, excludedContainers]);

    const selectedContainerSet = useMemo(() => new Set(selectedContainers), [selectedContainers]);

    const toggleContainerFilter = useCallback((container: string) => {
      setExcludedContainers((previous) =>
        previous.includes(container) ? previous.filter((excluded) => excluded !== container) : [...previous, container],
      );
    }, []);

    // Convert ANSI once per unique message, then cache.
    const renderedLogs = useMemo(() => {
      const ansiCache = new Map<string, string>();

      return normalizedLogs.map((log): RenderedLogEntry => {
        const parsed = containerFilteringEnabled ? parseLogContainerLabel(log.message) : { message: log.message };
        const message = parsed.message;
        let html = ansiCache.get(message);
        if (html === undefined) {
          html = ansiConverter.toHtml(message);
          ansiCache.set(message, html);
        }
        return { ...log, message, containerName: parsed.containerName, html };
      });
    }, [containerFilteringEnabled, normalizedLogs]);

    const filteredLogs = useMemo(() => {
      if (!containerFilteringEnabled) return renderedLogs;
      if (availableContainers.length === 0) return renderedLogs;

      return renderedLogs.filter((log) => log.containerName && selectedContainerSet.has(log.containerName));
    }, [availableContainers.length, containerFilteringEnabled, renderedLogs, selectedContainerSet]);

    // Virtual window.
    const virtual = useMemo(() => {
      const totalHeight = filteredLogs.length * ITEM_HEIGHT;
      const start = Math.max(0, Math.floor(scrollTop / ITEM_HEIGHT) - OVERSCAN);
      const end = Math.min(filteredLogs.length, Math.ceil((scrollTop + VIEWPORT_HEIGHT) / ITEM_HEIGHT) + OVERSCAN);
      return {
        totalHeight,
        start,
        end,
        paddingTop: start * ITEM_HEIGHT,
        paddingBottom: (filteredLogs.length - end) * ITEM_HEIGHT,
        items: filteredLogs.slice(start, end),
      };
    }, [filteredLogs, scrollTop]);

    const handleScroll = useCallback(() => {
      if (!scrollRef.current) return;
      const { scrollTop, scrollHeight, clientHeight } = scrollRef.current;
      setScrollTop(scrollTop);
      setIsAtBottom(scrollHeight - scrollTop <= clientHeight + 50);
    }, []);

    useLayoutEffect(() => {
      if (autoScroll && isAtBottom && scrollRef.current) {
        scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
      }
    }, [filteredLogs.length, autoScroll, isAtBottom, wrapLines, showTimestamps]);

    return (
      <div className="relative flex flex-col h-full border rounded-md overflow-hidden">
        {(enableTimestamps || availableContainers.length > 0 || allowWrap || onClear) && (
          <div className="absolute top-2 right-2 flex flex-col gap-2 z-10">
            {enableTimestamps && (
              <QuickAction
                label="Timestamps"
                icon={<Timer className="h-3.5 w-3.5" />}
                active={showTimestamps}
                onClick={() => setShowTimestamps(!showTimestamps)}
              />
            )}
            {availableContainers.length > 0 && (
              <LogContainerFilter
                containers={availableContainers}
                selectedContainers={selectedContainers}
                onToggle={toggleContainerFilter}
              />
            )}
            {allowWrap && (
              <QuickAction
                label="Wrap Lines"
                icon={<WrapText className="h-3.5 w-3.5" />}
                active={wrapLines}
                onClick={() => setWrapLines(!wrapLines)}
              />
            )}
            {onClear && (
              <QuickAction label="Clear Console" icon={<Eraser className="h-3.5 w-3.5" />} onClick={onClear} />
            )}
          </div>
        )}

        <div
          ref={scrollRef}
          onScroll={handleScroll}
          className={cn(
            'p-4 max-h-150 rounded-sm text-xs inline-block w-full overflow-auto bg-transparent',
            className,
          )}>
          {renderedLogs.length > 0 ? (
            <>
              {/* Top spacer */}
              <div style={{ height: virtual.paddingTop }} aria-hidden />
              {virtual.items.map((log, idx) => (
                <LogRow key={virtual.start + idx} log={log} showTimestamps={showTimestamps} wrapLines={wrapLines} />
              ))}
              {/* Bottom spacer */}
              <div style={{ height: virtual.paddingBottom }} aria-hidden />
            </>
          ) : (
            <div className="text-zinc-600 italic">No logs available...</div>
          )}
        </div>
      </div>
    );
  },
);
LogViewer.displayName = 'LogViewer';
function parseAnsiTerminalStream(rawText: string): string[] {
  if (!rawText) return [];

  // eslint-disable-next-line no-control-regex
  let cleanText = rawText.replace(/\x1b\[\?25[lh]/g, '').replace(/\?25[lh]/g, '');

  // Heal stripped Docker Compose cursor controls (e.g., AG[+] or G[+])
  // We capture any number of 'A' tokens (Cursor Up) followed by 'G' (Carriage Return)
  // right before the '[+]' progress block, and restore them to standard terminal controls.
  cleanText = cleanText.replace(/(^|[^\d[])(A*)G(?=\s*\[\+\])/g, (_, prefix, cursorUpChars) => {
    const structuralUps = '\x1b[A'.repeat(cursorUpChars.length);
    return prefix + structuralUps + '\r';
  });
  cleanText = collapseDockerComposeFrames(cleanText);

  const lines: string[] = [];
  let cursorLine = 0;

  // eslint-disable-next-line no-control-regex
  const tokens = cleanText.split(/(\r|\n|\x1b\[\d*A|\x1b\[\d*G)/);

  for (const token of tokens) {
    if (!token) continue;

    if (token === '\n') {
      cursorLine++;
      if (cursorLine >= lines.length) {
        lines.push('');
      }
    } else if (token === '\r' || (token.startsWith('\x1b[') && token.endsWith('G'))) {
      if (lines[cursorLine] !== undefined) {
        lines[cursorLine] = '';
      }
    } else if (token.startsWith('\x1b[') && token.endsWith('A')) {
      // eslint-disable-next-line no-control-regex
      const match = token.match(/\x1b\[(\d+)A/);
      const count = match ? parseInt(match[1], 10) : 1;
      cursorLine = Math.max(0, cursorLine - count);
    } else {
      if (lines[cursorLine] === undefined) {
        while (lines.length <= cursorLine) lines.push('');
      }
      lines[cursorLine] = token;
    }
  }

  while (lines.length > 0 && lines[lines.length - 1].trim() === '') {
    lines.pop();
  }

  return lines.map((line) => line.trimEnd());
}

function collapseDockerComposeFrames(value: string) {
  const output: string[] = [];
  let composeHeader: string | undefined;
  let composeRows = new Map<string, string>();

  const flushComposeFrame = () => {
    if (!composeHeader) return;
    output.push(composeHeader);
    output.push(...composeRows.values());
    composeHeader = undefined;
    composeRows = new Map<string, string>();
  };

  for (const rawLine of value.split('\n')) {
    const line = removeCursorControls(rawLine).trimEnd();
    const plainLine = stripAnsiCodes(line).trim();
    if (!plainLine) {
      continue;
    }

    if (plainLine.startsWith('[+]')) {
      composeHeader = line;
      continue;
    }

    const composeResourceKey = composeHeader ? getDockerComposeResourceKey(plainLine) : undefined;
    if (composeResourceKey) {
      composeRows.set(composeResourceKey, line);
      continue;
    }

    flushComposeFrame();
    output.push(line);
  }

  flushComposeFrame();
  return output.join('\n');
}

function getDockerComposeResourceKey(value: string) {
  const match = value.match(/\b(Container|Network|Volume|Image|Service)\s+(\S+)/);
  return match ? `${match[1]} ${match[2]}` : undefined;
}

function removeCursorControls(value: string) {
  return (
    value
      // eslint-disable-next-line no-control-regex
      .replace(/\x1b\[\d*[AG]/g, '')
      .replace(/\r/g, '')
      // Some malformed stream chunks can leave a stripped cursor-column sequence as ESC[0[+].
      // eslint-disable-next-line no-control-regex
      .replace(/\x1b\[\d*(?=\[\+\])/, '')
  );
}

function stripAnsiCodes(value: string) {
  // eslint-disable-next-line no-control-regex
  return value.replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, '');
}
export const MemoryUsageCell = ({
  state,
  stats,
}: {
  state: ContainerStateStatus;
  stats?: ContainerStatView | null;
}) => {
  if (state !== ContainerStateStatus.Running) {
    return <div className="text-muted">0B / 0B</div>;
  }
  return (
    <span className="text-[13px]">
      {byteTransform(stats?.memoryActive ?? 0, 2) + ' / ' + byteTransform(stats?.memoryLimit ?? 0, 2)}
    </span>
  );
};

export const CPUCell = ({ state, stats }: { state: ContainerStateStatus; stats?: ContainerStatView | null }) => {
  if (state !== ContainerStateStatus.Running) {
    return <div className="text-muted">0%</div>;
  }
  return (
    <span className="text-[13px]">{stats?.cpuUsage ? toFixedNumber(stats?.cpuUsage as number, 'percent') : '0%'}</span>
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
      <Server width={13} height={13} className={status === PlatformStatus.Online ? 'text-green-500' : 'text-red-500'} />
      <Link to={`/platforms/${id}`} className="table-link" title={name}>
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
}: {
  resourceType: ActivityResourceType | AlertResourceType;
  resourceId: string | undefined;
  resourceName: string | undefined;
}) => {
  const resourceConfig: Partial<Record<ActivityResourceType, { Icon: any; path: string }>> = {
    [ActivityResourceType.AlertRule]: { Icon: Megaphone, path: `/alert-rules/edit/${resourceId}` },
    [ActivityResourceType.Deployment]: { Icon: Rocket, path: `/deployments/edit/${resourceId}` },
    [ActivityResourceType.Registry]: { Icon: Cable, path: `/registries/edit/${resourceId}` },
    [ActivityResourceType.Platform]: { Icon: Server, path: `/platforms/edit/${resourceId}` },
    [ActivityResourceType.Stack]: { Icon: Layers, path: `/stacks/edit/${resourceId}` },
    [ActivityResourceType.GitRepository]: { Icon: GitBranch, path: `/git-repos/edit/${resourceId}` },
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

export const ActivityStatusCell = ({ status }: { status: ActivityStatus }) => {
  const statusConfig: Record<ActivityStatus, { className: string; label: string }> = {
    [ActivityStatus.Success]: { className: 'bg-green-200/25 text-green-700', label: 'Success' },
    [ActivityStatus.Information]: { className: 'bg-blue-200/25 text-blue-700', label: 'Info' },
    [ActivityStatus.Warning]: { className: 'bg-orange-200/25 text-orange-500', label: 'Warning' },
    [ActivityStatus.Failure]: { className: 'bg-red-200/25 text-red-700', label: 'Failure' },
  };

  const { className, label } = statusConfig[status] ?? statusConfig[ActivityStatus.Failure];

  return <Badge className={className}>{label}</Badge>;
};

export const SeverityStatusCell = ({ severity }: { severity: AlertSeverity }) => {
  const severityConfig: Record<AlertSeverity, { className: string; label: string }> = {
    [AlertSeverity.Info]: { className: 'bg-blue-200/25 text-blue-700', label: 'Info' },
    [AlertSeverity.Warning]: {
      className: 'bg-orange-200/25 text-orange-500',
      label: 'Warning',
    },
    [AlertSeverity.Critical]: {
      className: 'bg-red-200/25 text-red-700',
      label: 'Critical',
    },
  };

  const { className, label } = severityConfig[severity] ?? severityConfig[AlertSeverity.Info];

  return <Badge className={className}>{label}</Badge>;
};

export function AlertEventStatusCell({ status }: { status: AlertEventStatus }) {
  const config: Record<AlertEventStatus, { label: string; className: string }> = {
    [AlertEventStatus.Active]: { label: 'Active', className: 'bg-red-200/25 text-red-700' },
    [AlertEventStatus.Acknowledged]: {
      label: 'Acknowledged',
      className: 'bg-orange-200/25 text-orange-600',
    },
    [AlertEventStatus.Resolved]: {
      label: 'Resolved',
      className: 'bg-green-200/25 text-green-700',
    },
  };

  const value = config[status];
  return <Badge className={value.className}>{value.label}</Badge>;
}

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

type PagedDataTableProps<TData extends { id?: string | null }, TValue> = {
  columns: ColumnDef<TData, TValue>[];
  data: TData[];
  isLoading: boolean;
  query: { page: number; pageSize: number };
  setQuery: (patch: Partial<{ page: number; pageSize: number }>) => void;
  totalCount?: number | string | null;
  showPagination?: boolean;
  onSelectionChange?: (selectedRows: TData[]) => void;
  getRowId?: (row: TData) => string;
};

export function PagedDataTable<TData extends { id?: string | null }, TValue>({
  columns,
  data,
  isLoading,
  query,
  setQuery,
  totalCount,
  showPagination = true,
  onSelectionChange,
  getRowId,
}: PagedDataTableProps<TData, TValue>) {
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
      <ContentCard>
        <DataTable
          columns={columns}
          data={data}
          isLoading={isLoading}
          onSelectionChange={onSelectionChange}
          getRowId={getRowId}
        />
      </ContentCard>
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
}: {
  value: string;
  onChange: (value: string) => void;
  options: (string | { value: string; label: string; icon?: React.ComponentType<{ className?: string }> })[];
  placeholder: string;
  allLabel: string;
  allIcon?: React.ComponentType<{ className?: string }>;
  selectableLabel?: boolean;
  className?: string;
}) {
  return (
    <div className={cn('min-w-50', className)}>
      <Select value={value} onValueChange={onChange}>
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

export const pageSizeOptions = [
  { value: '25', label: '25' },
  { value: '50', label: '50' },
  { value: '100', label: '100' },
  { value: '200', label: '200' },
];
