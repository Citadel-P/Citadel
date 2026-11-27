import { useMemo } from 'react';
import { useSearchParams } from 'react-router';
import { StockFeatures, ColumnDef } from '@tanstack/react-table';
import { BackupCoverageStatus, ResourceBackupSummaryView } from '@/api/generated/api.types';
import { backupStatusOptions } from './coverage-status-options';
import { Shield } from 'lucide-react';
import { ResourceFilterChip, ResourceFilterDropdown } from '@/components/custom/resource-filter';
import { CommandGroup, CommandItem } from '@/components/ui/command';
import { Button } from '@/components/ui/button';
import { backupSummaryStatus, type BackupCoverageState } from './coverage';
import { ResourceBackupIndicator } from './resource-backup-indicator';
import { ResourceBackupSource } from './resource-backup-summary';

type CoveredResource = { backupCoverage?: BackupCoverageState | null };

function useBackupStatus() {
  const [searchParams, setSearchParams] = useSearchParams();
  const selected = useMemo(
    () =>
      backupStatusOptions
        .filter((option) => searchParams.getAll('backupStatus').includes(option.value))
        .map((option) => option.value),
    [searchParams],
  );
  const setStatuses = (values: string[]) => {
    setSearchParams((current) => {
      const next = new URLSearchParams(current);
      next.delete('backupStatus');
      backupStatusOptions
        .filter((option) => values.includes(option.value))
        .forEach((option) => next.append('backupStatus', option.value));
      return next;
    });
  };
  return { selected, setStatuses };
}

export function useBackupStatusFilter<T extends CoveredResource>(items: T[]) {
  const { selected } = useBackupStatus();
  return useMemo(
    () =>
      selected.length ? items.filter((item) => selected.includes(backupSummaryStatus(item.backupCoverage))) : items,
    [items, selected],
  );
}

export function BackupStatusFilter() {
  const { selected, setStatuses } = useBackupStatus();
  const available = backupStatusOptions.filter((option) => !selected.includes(option.value));
  return (
    <div className="flex min-w-0 flex-wrap items-center gap-2">
      {backupStatusOptions
        .filter((option) => selected.includes(option.value))
        .map((option) => (
          <ResourceFilterChip
            key={option.value}
            label={`Remove ${option.label} backup filter`}
            icon={<option.Icon className={`size-3.5 shrink-0 ${option.color}`} aria-hidden="true" />}
            onClick={() => setStatuses(selected.filter((value) => value !== option.value))}>
            {option.label}
          </ResourceFilterChip>
        ))}
      <ResourceFilterDropdown
        label="Backup Filter"
        icon={Shield}
        searchPlaceholder="Search backup statuses..."
        emptyMessage={available.length === 0 ? 'No more backup statuses.' : 'No backup statuses found.'}
        footer={
          selected.length > 0 && (
            <div className="border-t p-1">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="w-full justify-start"
                onClick={() => setStatuses([])}>
                Clear backup filters
              </Button>
            </div>
          )
        }>
        <CommandGroup>
          {available.map((option) => (
            <CommandItem
              key={option.value}
              value={option.label}
              onSelect={() => setStatuses([...selected, option.value])}
              className="cursor-pointer">
              <option.Icon className={`size-3.5 shrink-0 ${option.color}`} aria-hidden="true" />
              <span className="truncate">{option.label}</span>
            </CommandItem>
          ))}
        </CommandGroup>
      </ResourceFilterDropdown>
    </div>
  );
}

type BackupCellTarget = {
  coverage?: ResourceBackupSummaryView | null;
  source: ResourceBackupSource;
  returnTo: string;
};

export function ResourceBackupCell(value: BackupCellTarget) {
  if (backupSummaryStatus(value.coverage) === BackupCoverageStatus.NotApplicable) {
    return (
      <span className="text-muted-foreground" title="No eligible storage">
        —
      </span>
    );
  }
  return <ResourceBackupIndicator compact {...value} />;
}

export function backupColumn<T extends object>(
  target: (row: T) => BackupCellTarget | undefined,
): ColumnDef<StockFeatures, T> {
  return {
    id: 'backup',
    header: 'Backup',
    accessorFn: (row) => backupSummaryStatus(target(row)?.coverage),
    cell: ({ row }) => {
      const value = target(row.original);
      return value ? <ResourceBackupCell {...value} /> : null;
    },
  };
}
