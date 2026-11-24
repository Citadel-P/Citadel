import { useState, useCallback } from 'react';
import { Link } from 'react-router';
import { SearchField } from '@/components/custom/search-field';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import Loader from '@/components/ui/loader';
import { Table, TableHeader, TableHead, TableBody, TableRow, TableCell } from '@/components/ui/table';
import { useRead } from '@/lib/hooks';
import { CitadelIcons } from '@/lib/icons';
import type { RegularResourceComponents } from '@/pages/types';
import { audiences } from './visibility';
import { DashboardHomePreference } from './home-preference';
import { CreateDashboardDialog } from './create';

const pageSize = 50;
export const DashboardComponents: RegularResourceComponents = {
  Icon: CitadelIcons.Dashboard,
  showTaskSheet: false,
  header: { showAdd: false, showSearch: false },
  Content: DashboardList,
};

function DashboardList() {
  const [search, setSearch] = useState('');
  const [creating, setCreating] = useState(false);
  const [page, setPage] = useState(0);
  const searchBoards = useCallback(
    (value: string) => {
      setSearch(value);
      setPage(0);
    },
    [setPage, setSearch],
  );
  const list = useRead(
    'listDashboards',
    { query: { search, limit: pageSize + 1, offset: page * pageSize } },
    { staleTime: 0 },
  );
  const boards = list.isFetchedAfterMount ? (list.data?.data.dashboards ?? []) : [];
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <SearchField className="max-w-72" placeholder="Search dashboards…" onSearch={searchBoards} />
        <div className="flex flex-wrap items-center gap-2">
          <DashboardHomePreference />
          <Button onClick={() => setCreating(true)}>Create dashboard</Button>
        </div>
      </div>
      {creating && <CreateDashboardDialog close={() => setCreating(false)} />}
      {list.isError ? (
        <ResourceReadError error={list.error} refetch={list.refetch} isFetching={list.isFetching} />
      ) : !list.isFetchedAfterMount ? (
        <Loader />
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Dashboard</TableHead>
              <TableHead>Who can read</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {boards.length === 0 && (
              <TableRow>
                <TableCell colSpan={2}>No dashboards found.</TableCell>
              </TableRow>
            )}
            {boards.slice(0, pageSize).map((board) => (
              <TableRow key={board.id}>
                <TableCell>
                  <Link className="table-link" to={`/dashboards/${board.id}`}>
                    {board.name}
                  </Link>
                </TableCell>
                <TableCell>
                  <Badge variant="outline">{audiences[board.visibility]}</Badge>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
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
