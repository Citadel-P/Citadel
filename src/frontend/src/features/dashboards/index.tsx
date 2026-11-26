import { DashboardsTable } from './table';
import { useState } from 'react';
import { Plus } from 'lucide-react';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { Button } from '@/components/ui/button';
import { useRead } from '@/lib/hooks';
import { CitadelIcons } from '@/lib/icons';
import type { RegularResourceComponents } from '@/pages/types';
import { DashboardHomePreference } from './home-preference';
import { CreateDashboardDialog } from './create';

const pageSize = 50;
export const DashboardComponents: RegularResourceComponents = {
  Icon: CitadelIcons.Dashboard,
  showTaskSheet: false,
  header: {
    showAdd: false,
    showSearch: true,
    Actions: CreateDashboardButton,
    Extra: DashboardHomePreference,
  },
  Content: ({ search = '' }) => <DashboardList key={search} search={search} />,
};

function CreateDashboardButton() {
  const [creating, setCreating] = useState(false);
  return (
    <>
      <Button onClick={() => setCreating(true)}>
        <Plus />
        Create dashboard
      </Button>
      {creating && <CreateDashboardDialog close={() => setCreating(false)} />}
    </>
  );
}

function DashboardList({ search }: { search: string }) {
  const [page, setPage] = useState(0);
  const list = useRead(
    'listDashboards',
    { query: { search, limit: pageSize + 1, offset: page * pageSize } },
    { staleTime: 0 },
  );
  const boards = list.isFetchedAfterMount ? (list.data?.data.dashboards ?? []) : [];
  return (
    <div className="space-y-4">
      {list.isError ? (
        <ResourceReadError error={list.error} refetch={list.refetch} isFetching={list.isFetching} />
      ) : (
        <DashboardsTable key={page} items={boards.slice(0, pageSize)} isLoading={!list.isFetchedAfterMount} />
      )}
      {(page > 0 || boards.length > pageSize) && (
        <nav aria-label="Dashboard pages" className="flex justify-center items-center gap-3">
          <Button variant="outline" size="sm" disabled={page === 0} onClick={() => setPage(page - 1)}>
            Previous
          </Button>
          <span>Page {page + 1}</span>
          <Button variant="outline" size="sm" disabled={boards.length <= pageSize} onClick={() => setPage(page + 1)}>
            Next
          </Button>
        </nav>
      )}
    </div>
  );
}
