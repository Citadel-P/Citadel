import { DataTable } from '@/components/ui/data-table';
import { AlertRuleView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';
import { useSelectedResources } from '@/lib/atoms';
import { ContentCard } from '@/components/custom/content-card';
import { SeverityStatusCell } from '@/components/custom/common';
import { Activity, Clock, Zap } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Checkbox } from '@/components/ui/checkbox';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useNavigate } from 'react-router';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';

const EMPTY_ROWS: AlertRuleView[] = [];

export const AlertRulesTable = ({
  actions,
  items,
  isLoading,
}: {
  items: AlertRuleView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: AlertRuleView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [_, setSelectedResources] = useSelectedResources<AlertRuleView>('AlertRule');
  const cols = useMemo(() => columns(actions ?? {}), [actions]);

  return (
    <ContentCard>
      <DataTable
        columns={cols}
        data={items ?? EMPTY_ROWS}
        isLoading={isLoading}
        onSelectionChange={setSelectedResources}
      />
    </ContentCard>
  );
};

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: AlertRuleView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
): ColumnDef<AlertRuleView>[] => {
  const cols: ColumnDef<AlertRuleView>[] = [
    {
      id: 'select',
      header: ({ table }) => (
        <Checkbox
          checked={table.getIsAllPageRowsSelected() || (table.getIsSomePageRowsSelected() && 'indeterminate')}
          onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
          aria-label="Select all"
        />
      ),
      cell: ({ row }) => (
        <Checkbox
          checked={row.getIsSelected()}
          onCheckedChange={(value) => row.toggleSelected(!!value)}
          aria-label="Select alert rule"
        />
      ),
      enableSorting: false,
      enableHiding: false,
    },
    {
      accessorKey: 'name',
      header: ({ column }) => <SortableCell cellName="Name" column={column} />,
      cell: ({ row }) => <RuleNameRow alertRule={row.original} />,
      sortingFn: (rowA: any, rowB: any): number => rowA.original?.name?.localeCompare(rowB.original?.name),
    },
    {
      accessorKey: 'severity',
      header: ({ column }) => <SortableCell cellName="Severity" column={column} />,
      cell: ({ row }) => <SeverityStatusCell severity={row.original.severity} />,
      sortingFn: (rowA: any, rowB: any): number => rowA.original?.severity?.localeCompare(rowB.original?.severity),
    },
    {
      accessorKey: 'conditions',
      header: ({ column }) => <SortableCell cellName="Conditions" column={column} />,
      cell: ({ row }) => <RuleConditionCell rule={row.original} />,
      sortingFn: (rowA: any, rowB: any): number => rowA.original?.threshold?.localeCompare(rowB.original?.threshold),
    },
    {
      accessorKey: 'channels',
      header: ({ column }) => <SortableCell cellName="Channels" column={column} />,
      cell: ({ row }) => <ChannelsCell rule={row.original} />,
    },
    {
      id: 'actions',
      cell: ({ row }) => <RowActionMenu resource={row.original as any} actions={actions} />,
    },
  ];

  return cols;
};

function RuleConditionCell({ rule }: { rule: AlertRuleView }) {
  return (
    <div className="py-3">
      <div className="flex flex-col gap-1.5">
        {rule.threshold ? (
          <div className="flex items-center gap-2 ">
            <Activity className="h-3.5 w-3.5 text-muted-foreground" />
            <span className="font-medium text-xs">
              &ge; {rule.threshold}%<span className="text-muted-foreground font-normal mx-1">for</span>
              {rule.requiredMatches}x
            </span>
          </div>
        ) : (
          <div className="flex items-center gap-2 text-muted-foreground">
            <Zap className="h-3.5 w-3.5" />
            <span className="text-xs">Event Trigger</span>
          </div>
        )}
        {rule.cooldownSeconds && (
          <div className="flex items-center gap-2 text-muted-foreground">
            <Clock className="h-3.5 w-3.5 text-muted-foreground" />
            <span className="text-xs">{rule.cooldownSeconds}s cooldown</span>
          </div>
        )}
      </div>
    </div>
  );
}

function ChannelsCell({ rule }: { rule: AlertRuleView }) {
  return (
    <div className="flex -space-x-2">
      {rule.channels.map((c, i) => (
        <div
          key={i}
          className={cn(
            'h-8 w-8 rounded-full border-2 border-background flex items-center justify-center text-white text-xs font-normal shadow-sm relative z-10 transition-transform hover:scale-110 hover:z-20 cursor-help',
            c.alertDestination === 'Slack'
              ? 'bg-[#4A154B]'
              : c.alertDestination === 'Discord'
                ? 'bg-[#5865F2]'
                : c.alertDestination === 'Generic'
                  ? 'bg-blue-600'
                  : 'bg-zinc-900',
          )}
          title={`${c.alertDestination}: ${c.name}`}>
          {c.alertDestination[0]}
        </div>
      ))}
      {rule.channels.length === 0 && (
        <span className="text-xs text-muted-foreground italic py-1 px-2 ">No channels</span>
      )}
    </div>
  );
}

const RuleNameRow = ({ alertRule }: { alertRule: AlertRuleView }) => {
  const navigate = useNavigate();
  function onClick() {
    navigate(`/alert-rules/edit/${alertRule.id}/`);
  }
  return (
    <div className="flex items-center whitespace-nowrap">
      <div className="flex items-center">
        <StateIndicator value={alertRule.status} />
      </div>
      <span
        className="cursor-pointer hover:underline"
        onClick={onClick}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            onClick();
          }
        }}
        tabIndex={0}
        role="button"
        aria-label="Show alert rule details">
        {alertRule.name}
      </span>
    </div>
  );
};
