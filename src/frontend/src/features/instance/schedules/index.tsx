import { useState } from 'react';
import { Layers } from 'lucide-react';
import { ResourceMultiFilter } from '@/components/custom/resource-filter';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import { SearchField } from '@/components/custom/search-field';
import { Table, TableHeader, TableRow, TableHead, TableBody, TableCell } from '@/components/ui/table';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { instanceResourceOption } from '../resource-options';
import { ScheduleGroup } from './group';
import { purposes } from './presentation';
import { useSchedules } from './useSchedules';

const entries = Object.entries(purposes);
const resourceOptions = [...new Set(entries.map(([, [resource]]) => resource))].sort().map(instanceResourceOption);

export function InstanceSchedules() {
  const [resources, setResources] = useState<string[]>([]);
  const [name, setName] = useState('');
  const [attentionOnly, setAttentionOnly] = useState(false);
  const search = name.trim();
  const { query, activeBatches, synchronized } = useSchedules(search);
  const formatDateTime = useProfileDateTimeFormatter();
  const groups = new Map<string, { label: string; total: number; attention: number }>();
  for (const { purpose, resource, count, attention } of query.isError ? [] : (query.data?.data ?? [])) {
    const label = purposes[purpose]?.[0] ?? resource;
    if (!resources.length || resources.includes(label)) {
      const blocked = Number(attention ?? 0);
      const group = groups.get(resource) ?? { label, total: 0, attention: 0 };
      group.total += Number(count);
      group.attention += blocked;
      groups.set(resource, group);
    }
  }
  const visibleGroups = [...groups].filter(([, group]) => !attentionOnly || group.attention > 0);
  return (
    <section className="space-y-4" aria-label="Resource schedules">
      <div
        role="region"
        aria-label="Schedule filters"
        className="flex min-w-0 flex-wrap items-center gap-2 rounded-lg border bg-card p-3 shadow-xs">
        <SearchField placeholder="Search schedules by name…" className="mb-0 w-full sm:w-72" onSearch={setName} />
        <div className="flex min-w-0 flex-wrap items-center gap-2 sm:ml-auto">
          <div className="inline-flex h-9 items-center gap-2 rounded-sm border bg-background px-2.5">
            <Switch id="schedule-attention" checked={attentionOnly} onCheckedChange={setAttentionOnly} />
            <Label htmlFor="schedule-attention" className="text-xs font-normal">
              Needs attention
            </Label>
          </div>
          <ResourceMultiFilter
            label="Resource"
            icon={Layers}
            selected={resources}
            options={resourceOptions}
            onChange={setResources}
          />
        </div>
      </div>

      <Table aria-label="Resource schedules">
        <TableHeader>
          <TableRow>
            {[
              'Resource',
              'Purpose',
              'Recurrence',
              'Time zone',
              'Next scheduled time',
              'Schedule state',
              'Latest scheduled run',
            ].map((label) => (
              <TableHead key={label}>{label}</TableHead>
            ))}
          </TableRow>
        </TableHeader>
        {visibleGroups
          .sort(([, a], [, b]) => a.label.localeCompare(b.label))
          .map(([resource, { label, total, attention }]) => (
            <ScheduleGroup
              key={JSON.stringify([resource, search, attentionOnly])}
              resource={resource}
              label={label}
              synchronized={synchronized}
              total={total}
              attention={attention}
              attentionOnly={attentionOnly}
              search={search}
              filtered={attentionOnly || !!resources.length || !!search}
              activeBatches={activeBatches.filter((p) => purposes[p]?.[0] === label)}
              formatDateTime={formatDateTime}
            />
          ))}
        {!visibleGroups.length && (
          <TableBody>
            <TableRow>
              <TableCell colSpan={7} className="py-8 text-center text-muted-foreground">
                {query.isError
                  ? 'Schedule catalog unavailable.'
                  : query.isPending
                    ? 'Loading schedules…'
                    : attentionOnly
                      ? 'No blocked schedules match your filters.'
                      : resources.length || search
                        ? 'No schedules match your filters.'
                        : 'No schedules found.'}
              </TableCell>
            </TableRow>
          </TableBody>
        )}
      </Table>
    </section>
  );
}
