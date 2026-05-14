import { PagedResultViewOfTeamView, TeamView } from '@/api/generated/api.types';
import { ActionData, DropdownActionComponent } from '@/pages/types';
import { useSelectedResources, useTeamQuery } from '@/lib/atoms';
import { useMemo } from 'react';
import { PagedDataTable } from '@/components/custom/common';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import SortableCell from '@/components/custom/sortable-cell';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Link } from 'react-router';

export const Teams = ({
  items,
  actions,
  isLoading,
}: {
  items: PagedResultViewOfTeamView;
  actions: Record<string, DropdownActionComponent>;
  isLoading: boolean;
}) => <TeamsTable pagedResult={items} isLoading={isLoading} actions={actions} />;

const EMPTY_ROWS: TeamView[] = [];

export const TeamsTable = ({
  pagedResult,
  actions,
  isLoading,
}: {
  pagedResult: PagedResultViewOfTeamView | undefined;
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: TeamView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [query, setQuery] = useTeamQuery();
  const [_, setSelectedResources] = useSelectedResources<TeamView>('Team');
  const cols = useMemo(() => columns(actions ?? {}), [actions]);

  return (
    <PagedDataTable
      columns={cols}
      data={pagedResult?.items ?? EMPTY_ROWS}
      isLoading={isLoading}
      query={query}
      setQuery={setQuery}
      totalCount={pagedResult?.totalCount}
      onSelectionChange={setSelectedResources}
    />
  );
};

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: TeamView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
): ColumnDef<TeamView>[] => [
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
        aria-label="Select team"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <UserNameRow team={row.original} />,
    sortingFn: (rowA, rowB) => String(rowA.original.name).localeCompare(String(rowB.original.name)),
  },
  {
    accessorKey: 'members',
    header: ({ column }) => <SortableCell cellName="Members" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{row.original.totalMembers}</span>,
  },
  {
    accessorKey: 'roles',
    header: ({ column }) => <SortableCell cellName="Roles" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{row.original.roles?.map((s) => s.name).join(' + ')}</span>,
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

const UserNameRow = ({ team }: { team: TeamView }) => {
  return (
    <div className="flex items-center gap-1">
      <StateIndicator enableLabel={team.isEnabled} value={team.isEnabled} />
      <Link to={`../access/teams/edit/${team.id}`} className="hover:underline">
        {team.name}
      </Link>
    </div>
  );
};
