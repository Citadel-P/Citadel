import { useCallback, useMemo } from 'react';
import { Link } from 'react-router';
import type { StockFeatures, ColumnDef, Row } from '@tanstack/react-table';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { DataTable } from '@/components/ui/data-table';
import { AlertRuleStatus, AlertType, type AlertRuleListItem } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { useSelectedResources } from '@/lib/atoms';
import { StateBadge } from '@/components/custom/state-badge';
import { Checkbox } from '@/components/ui/checkbox';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import type { ActionData } from '@/pages/types';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { useAlertRuleFilters } from './filters';
import { groupRules, isRuleRow, resourceLabel, type RuleTableRow } from './presentation';

type RuleActions = Record<
  string,
  React.FC<{ resource: AlertRuleListItem; onAction?: (actionKey: string, actionData?: ActionData) => void }>
>;
const EMPTY_ROWS: AlertRuleListItem[] = [];

export function AlertRulesTable({
  actions,
  items = EMPTY_ROWS,
  isLoading,
}: {
  items: AlertRuleListItem[];
  isLoading: boolean;
  actions: RuleActions;
}) {
  const [, setSelected] = useSelectedResources<AlertRuleListItem>('AlertRule');
  const { value: filters } = useAlertRuleFilters();
  const rows = useMemo(() => groupRules(items, filters), [items, filters]);
  const cols = useMemo(() => columns(actions ?? {}), [actions]);
  const getSubRows = useCallback((row: RuleTableRow) => (row.kind === 'group' ? row.rules : undefined), []);
  const onSelectionChange = useCallback(
    (rows: RuleTableRow[]) => setSelected(rows.filter(isRuleRow).map((row) => row.rule)),
    [setSelected],
  );
  return (
    <DataTable<RuleTableRow>
      expansionStorageKey="alert-rules-expanded"
      columns={cols}
      data={rows}
      isLoading={isLoading}
      getSubRows={getSubRows}
      getRowClassName={(row) =>
        row.depth > 0 ? 'bg-muted/30 even:bg-muted/30 hover:bg-muted/50 data-[state=selected]:bg-muted/60' : undefined
      }
      enableRowSelection={isRuleRow}
      enableSubRowSelection={true}
      onSelectionChange={onSelectionChange}
      emptyState={{ title: items.length ? 'No rules match your filters.' : 'No alert rules.' }}
    />
  );
}

function columns(actions: RuleActions): ColumnDef<StockFeatures, RuleTableRow>[] {
  return [
    {
      id: 'select',
      header: ({ table }) => (
        <Checkbox
          checked={table.getIsAllPageRowsSelected() || (table.getIsSomePageRowsSelected() && 'indeterminate')}
          onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
          aria-label="Select all rules"
        />
      ),
      cell: ({ row }) => (
        <Checkbox
          checked={
            isRuleRow(row.original)
              ? row.getIsSelected()
              : row.getIsAllSubRowsSelected() || (row.getIsSomeSelected() && 'indeterminate')
          }
          onCheckedChange={(value) => row.toggleSelected(!!value)}
          aria-label={
            isRuleRow(row.original)
              ? `Select ${row.original.rule.name}`
              : `Select ${resourceLabel(row.original.resourceType)} rules`
          }
        />
      ),
      enableSorting: false,
      enableHiding: false,
    },
    {
      id: 'name',
      accessorFn: (row) => (isRuleRow(row) ? row.rule.name : resourceLabel(row.resourceType)),
      header: ({ column }) => <SortableCell cellName="Name" column={column} />,
      cell: ({ row }) => <RuleNameCell row={row} />,
    },
    {
      id: 'severity',
      accessorFn: (row) => (isRuleRow(row) ? row.rule.severity : ''),
      header: ({ column }) => <SortableCell cellName="Severity" column={column} />,
      cell: ({ row }) =>
        isRuleRow(row.original) ? <StateBadge value={row.original.rule.severity} kind="alertSeverity" /> : null,
    },
    {
      id: 'conditions',
      header: 'Conditions',
      enableSorting: false,
      cell: ({ row }) =>
        isRuleRow(row.original) ? (
          <RuleConditionCell rule={row.original.rule} />
        ) : (
          <span className="text-xs text-muted-foreground">
            {row.original.enabled} enabled · {row.original.total - row.original.enabled} disabled
          </span>
        ),
    },
    {
      id: 'channels',
      header: 'Channels',
      enableSorting: false,
      cell: ({ row }) => (isRuleRow(row.original) ? <ChannelsCell rule={row.original.rule} /> : null),
    },
    {
      id: 'actions',
      header: () => <span className="sr-only">Actions</span>,
      enableSorting: false,
      cell: ({ row }) =>
        isRuleRow(row.original) ? <RowActionMenu resource={row.original.rule} actions={actions} /> : null,
    },
  ];
}

function RuleNameCell({ row }: { row: Row<StockFeatures, RuleTableRow> }) {
  const item = row.original;
  if (isRuleRow(item))
    return (
      <div className="flex min-w-0 items-center gap-2 pl-7">
        <StateIndicator value={item.rule.status} />
        <Link className="table-link block truncate font-normal" to={`/alert-rules/edit/${item.rule.id}/`}>
          {item.rule.name}
        </Link>
      </div>
    );
  const state =
    item.enabled === item.total ? AlertRuleStatus.Enabled : item.enabled === 0 ? AlertRuleStatus.Disabled : 'Mixed';
  return (
    <div className="flex min-w-0 items-center gap-2">
      <button
        type="button"
        className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-muted-foreground hover:bg-accent/70 hover:text-foreground"
        aria-label={`${row.getIsExpanded() ? 'Collapse' : 'Expand'} ${resourceLabel(item.resourceType)} rules`}
        aria-expanded={row.getIsExpanded()}
        onClick={row.getToggleExpandedHandler()}>
        {row.getIsExpanded() ? (
          <ChevronDown aria-hidden="true" className="h-3.5 w-3.5" />
        ) : (
          <ChevronRight aria-hidden="true" className="h-3.5 w-3.5" />
        )}
      </button>
      <StateIndicator value={state} tooltip={`${item.enabled} enabled · ${item.total - item.enabled} disabled`} />
      <button
        type="button"
        className="truncate cursor-pointer text-left text-sm font-medium text-foreground hover:underline"
        aria-label={`${resourceLabel(item.resourceType)} rules`}
        aria-expanded={row.getIsExpanded()}
        onClick={row.getToggleExpandedHandler()}>
        {resourceLabel(item.resourceType)}
      </button>
      <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
        {item.rules.length === item.total ? item.total : `${item.rules.length}/${item.total}`}{' '}
        {item.total === 1 ? 'rule' : 'rules'}
      </span>
    </div>
  );
}

function RuleConditionCell({ rule }: { rule: AlertRuleListItem }) {
  return (
    <span className="text-xs text-muted-foreground">
      {rule.type === AlertType.BuildAgentPoolUnavailable
        ? `Unavailable ≥ ${rule.threshold ?? 90}s`
        : rule.threshold != null
          ? `≥ ${rule.threshold}%${rule.requiredMatches != null ? ` for ${rule.requiredMatches} matches` : ''}`
          : 'Event trigger'}
      {rule.cooldownSeconds != null && rule.cooldownSeconds > 0 ? ` · ${rule.cooldownSeconds}s cooldown` : ''}
    </span>
  );
}

function ChannelsCell({ rule }: { rule: AlertRuleListItem }) {
  if (!rule.channels.length) return <span className="text-xs text-muted-foreground">No channels</span>;
  return (
    <Tooltip>
      <TooltipTrigger type="button" className="inline-block max-w-48 truncate align-middle text-xs font-normal">
        {rule.channels[0].name}
        {rule.channels.length > 1 ? ` +${rule.channels.length - 1}` : ''}
      </TooltipTrigger>
      <TooltipContent>
        <ul>
          {rule.channels.map((channel) => (
            <li key={channel.id}>
              {channel.name} · {channel.alertDestination}
              {!channel.isActive ? ' · Disabled' : ''}
            </li>
          ))}
        </ul>
      </TooltipContent>
    </Tooltip>
  );
}
