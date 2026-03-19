import { useMemo, useState, useLayoutEffect, useRef, useCallback, useEffect } from 'react';
import { ColumnDef } from '@tanstack/react-table';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import {
  ActorType,
  ActivityStatus,
  AutoUpdateStatus,
  ContainerStateStatus,
  ContainerStatView,
  PlatformStatus,
  UpdateBehavior,
  ActivityResourceType,
  AlertSeverity,
  AlertResourceType,
} from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Command, CommandInput, CommandList, CommandGroup, CommandItem, CommandEmpty } from '@/components/ui/command';
import { DataTable } from '@/components/ui/data-table';
import {
  Cable,
  Check,
  ChevronDown,
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
import { cn, filterBySplit, toFixedNumber } from '@/lib/utils';
import { PluralResourceMap, ResourceType } from '@/api/types';
import { useMeasuredWidth, useRead, useLocalStorage } from '@/lib/hooks';
import { useResourceFilter } from '@/lib/atoms';
import Convert from 'ansi-to-html';
import { Badge } from '../ui/badge';
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
  Eraser,
  RefreshCcw,
  RefreshCcwDot,
  RefreshCwOff,
  Timer,
  WrapText,
} from 'lucide-react';
import { byteTransform } from '@/lib/bytes.helper';
import { Link } from 'react-router';
import { Select, SelectContent, SelectItem, SelectSeparator, SelectTrigger, SelectValue } from '@/components/ui/select';

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

  const read = useRead(`list${resourceName}` as any, { platformId });
  const items = (Object.values(read.data?.data ?? {}).at(0) as T[]) ?? [];
  const selectedItem =
    filter?.item ?? (typeof selected === 'string' ? items.find((i) => i.id === selected) : selected) ?? undefined;

  const filtered = filterBySplit(items, search, (i) => i.name).sort((a, b) => a.name.localeCompare(b.name));
  useEffect(() => {
    setFilter(null);
  }, [selected]);
  const handleSelect = (item: T | undefined) => {
    setFilter(item ? { item } : null);
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
            'flex justify-between gap-2 w-full max-w-[400px] font-normal data-[placeholder]:text-muted-foreground text-sm bg-background hover:bg-background shadow-xs border',
            className,
          )}>
          {defaultDisplay(selectedItem) ?? placeholder}
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
  type,
  selected,
  onSelect,
  disabled,
  placeholder,
  platformId,
  className,
  valueKey = 'id',
}: {
  type: ResourceType;
  selected?: string[] | T[];
  onSelect?: (items: T[]) => void;
  disabled?: boolean;
  align?: 'start' | 'center' | 'end';
  placeholder?: string;
  platformId?: string;
  className?: string;
  valueKey?: 'id' | 'name';
}) {
  const resourceName = PluralResourceMap[type];

  const read = useRead(`list${resourceName}` as any, { platformId });
  const items = (Object.values(read.data?.data ?? {}).at(0) as T[]) ?? [];

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

const convert = new Convert({
  newline: false,
  escapeXML: true,
});

export interface LogEntry {
  timestamp?: string;
  message: string;
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

export const LogViewer = ({
  logs,
  autoScroll = true,
  className,
  showTimestamps: initialShowTimestamps = false,
  wrapLines: initialWrapLines = true,
  timeStamps: enableTimestamps = false,
  allowWrap = false,
  onClear,
}: LogViewerProps) => {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [isAtBottom, setIsAtBottom] = useState(true);
  const [showTimestamps, setShowTimestamps] = useLocalStorage('log-viewer-show-timestamps', initialShowTimestamps);
  const [wrapLines, setWrapLines] = useLocalStorage('log-viewer-wrap-lines', initialWrapLines);

  const normalizedLogs = useMemo((): LogEntry[] => {
    if (!logs) return [];

    // Handle single string (e.g., from a simple text fetch)
    if (typeof logs === 'string') {
      return logs.split('\n').map((line) => ({ message: line }));
    }

    // Handle array
    return logs.map((log) => {
      // If it's already an object {timestamp, message}, keep it
      if (typeof log === 'object' && log !== null) {
        return log;
      }
      // If it's an array of strings (e.g., Deploy Progress)
      return { message: log };
    });
  }, [logs]);

  const handleScroll = () => {
    if (!scrollRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollRef.current;
    setIsAtBottom(scrollHeight - scrollTop <= clientHeight + 50);
  };

  useLayoutEffect(() => {
    if (autoScroll && isAtBottom && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [normalizedLogs, autoScroll, isAtBottom, wrapLines, showTimestamps]);

  return (
    <div className="relative flex flex-col h-full border rounded-md overflow-hidden">
      {(enableTimestamps || allowWrap || onClear) && (
        <div className="absolute top-2 right-2 flex flex-col gap-2 z-10">
          {enableTimestamps && (
            <QuickAction
              label="Timestamps"
              icon={<Timer className="h-3.5 w-3.5" />}
              active={showTimestamps}
              onClick={() => setShowTimestamps(!showTimestamps)}
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
          {onClear && <QuickAction label="Clear Console" icon={<Eraser className="h-3.5 w-3.5" />} onClick={onClear} />}
        </div>
      )}
      <div
        ref={scrollRef}
        onScroll={handleScroll}
        className={cn(
          'p-4 max-h-[600px] rounded-sm text-xs inline-block w-full overflow-auto bg-transparent',
          className,
        )}>
        {normalizedLogs.length > 0 ? (
          normalizedLogs.map((log, index) => (
            <div
              key={index}
              className={cn(
                'flex gap-2 min-h-[1.2rem]',
                wrapLines ? 'whitespace-pre-wrap break-all' : 'whitespace-pre',
              )}>
              {showTimestamps && log.timestamp && (
                <span className="text-foreground/30 shrink-0 select-none tabular-nums">
                  {log.timestamp.includes('T') ? log.timestamp.split('T')[1].slice(0, 8) : log.timestamp}
                </span>
              )}
              <span dangerouslySetInnerHTML={{ __html: convert.toHtml(log.message) }} />
            </div>
          ))
        ) : (
          <div className="text-zinc-600 italic">No logs available...</div>
        )}
      </div>
    </div>
  );
};

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
      <Link to={`/platforms/${id}`} className="table-link">
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
    [AlertSeverity.Info]: { className: 'bg-blue-200/25 text-blue-700 border-blue-500/20', label: 'Info' },
    [AlertSeverity.Warning]: {
      className: 'bg-orange-200/25 text-orange-500 border-orange-500/20',
      label: 'Warning',
    },
    [AlertSeverity.Critical]: {
      className: 'bg-red-200/25 text-red-700 border-red-500/20',
      label: 'Critical',
    },
  };

  const { className, label } = severityConfig[severity] ?? severityConfig[AlertSeverity.Info];

  return <Badge className={className}>{label}</Badge>;
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
    <div className={cn('min-w-[200px]', className)}>
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
