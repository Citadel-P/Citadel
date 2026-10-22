import { useContext, useMemo } from 'react';
import { RealtimeContext } from '@/lib/context/realtime-context';
import { useAtomValue, useAtom } from 'jotai';
import { atomWithStorage } from 'jotai/utils';
import { useSearchParams } from 'react-router';
import { Activity, Columns3 } from 'lucide-react';
import type { StockFeatures, ColumnDef } from '@tanstack/react-table';
import { ResourceFilterChip, ResourceFilterDropdown } from '@/components/custom/resource-filter';
import { CommandGroup, CommandItem } from '@/components/ui/command';
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuCheckboxItem,
} from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import SortableCell from '@/components/custom/sortable-cell';
import { healthLabels, healthTone, useHealthClock } from '@/components/custom/health-status';
import { WorkloadHealthCard } from './card';
import { type HealthResource, healthOrder, matchesHealth, workloadHealthStatus } from './health';

export type HealthResourceKind = 'Deployment' | 'Stack' | 'SwarmService';
const columns = {
  Deployment: atomWithStorage('deployments-health-column', true),
  Stack: atomWithStorage('stacks-health-column', true),
  SwarmService: atomWithStorage('swarm-services-health-column', true),
};
export function useHealthColumn(kind: HealthResourceKind) {
  return useAtomValue(columns[kind]);
}
export function WorkloadHealthColumns({ kind }: { kind: HealthResourceKind }) {
  const [visible, setVisible] = useAtom(columns[kind]);
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" className="ml-auto">
          <Columns3 className="size-3.5" />
          Columns
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuCheckboxItem
          checked={visible}
          onSelect={(event) => event.preventDefault()}
          onCheckedChange={setVisible}>
          Health
        </DropdownMenuCheckboxItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
export function useWorkloadHealthFilter<T extends HealthResource>(items: T[], kind: HealthResourceKind): T[] {
  const [params] = useSearchParams();
  const visible = useHealthColumn(kind);
  const now = useHealthClock();
  const live = useContext(RealtimeContext);
  const connected = !live || live.liveConnectionState === 'connected';
  const selection = visible ? params.getAll('health').join(',') : '';
  return useMemo(
    () => items.filter((item) => matchesHealth(item, selection ? selection.split(',') : [], now, connected)),
    [items, selection, now, connected],
  );
}
export function WorkloadHealthFilter({ kind }: { kind: HealthResourceKind }) {
  const visible = useHealthColumn(kind);
  const [params, setParams] = useSearchParams();
  const selected = params.getAll('health');
  const update = (values: string[]) =>
    setParams((previous) => {
      const next = new URLSearchParams(previous);
      next.delete('health');
      values.forEach((v) => next.append('health', v));
      return next;
    });
  if (!visible) return null;
  return (
    <div className="flex flex-wrap items-center gap-2">
      {healthOrder
        .filter((status) => selected.includes(status))
        .map((status) => (
          <ResourceFilterChip
            key={status}
            label={`Remove ${healthLabels[status]} health filter`}
            icon={<Activity className={`size-3.5 ${healthTone(status)}`} />}
            onClick={() => update(selected.filter((s) => s !== status))}>
            {healthLabels[status]}
          </ResourceFilterChip>
        ))}
      <ResourceFilterDropdown
        label="Health Filter"
        icon={Activity}
        searchPlaceholder="Search health states..."
        emptyMessage="No more health states.">
        <CommandGroup>
          {healthOrder
            .filter((status) => !selected.includes(status))
            .map((status) => (
              <CommandItem key={status} value={healthLabels[status]} onSelect={() => update([...selected, status])}>
                <Activity className={`size-3.5 ${healthTone(status)}`} />
                {healthLabels[status]}
              </CommandItem>
            ))}
        </CommandGroup>
      </ResourceFilterDropdown>
    </div>
  );
}
export function workloadHealthColumn<T extends HealthResource>(kind: HealthResourceKind): ColumnDef<StockFeatures, T> {
  return {
    id: 'probeHealth',
    accessorFn: (row) => workloadHealthStatus(row),
    header: ({ column }) => <SortableCell cellName="Health" column={column} />,
    cell: ({ row }) => <WorkloadHealthCard resource={row.original} kind={kind} />,
    sortFn: (a, b) =>
      healthOrder.indexOf(workloadHealthStatus(a.original)) - healthOrder.indexOf(workloadHealthStatus(b.original)),
  };
}
