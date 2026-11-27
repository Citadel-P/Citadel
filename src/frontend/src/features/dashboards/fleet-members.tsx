import { useDashboardRead } from './use-dashboard-read';
import { useCallback, useState } from 'react';
import { ArrowUpRight, CalendarClock, EthernetPort, Layers, Server } from 'lucide-react';
import { Link } from 'react-router';
import { DashboardStateKind, type DashboardFleetRow } from '@/api/generated/api.types';
import { PaginationControls } from '@/components/custom/common';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { SearchField } from '@/components/custom/search-field';
import { StateBadge } from '@/components/custom/state-badge';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Timestamp } from '@/components/custom/timestamp';
import { SheetDescription, SheetFooter, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import Loader from '@/components/ui/loader';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { sourcePath } from './source-path';

const pageSize = 25;
const metaClass = 'inline-flex min-w-0 items-center gap-1.5 [&_svg]:size-3.5 [&_svg]:shrink-0';

function MemberRow({ row }: { row: DashboardFleetRow }) {
  const format = useProfileDateTimeFormatter();
  const kind = Object.values(DashboardStateKind).find((kind) => kind === row.kind);
  const path = kind && (kind !== 'Container' || row.platformId) ? sourcePath(kind, row) : undefined;
  const badgeKind = kind === 'Container' ? 'container' : undefined;
  return (
    <li className="min-w-0 px-4 py-3">
      <div className="flex items-start justify-between gap-3">
        {path ? (
          <Link to={path} className="table-link group inline-flex min-w-0 items-center gap-1.5 text-sm font-medium">
            <span title={row.name} className="truncate">
              {row.name}
            </span>
            <ArrowUpRight
              aria-hidden="true"
              className="size-3.5 shrink-0 text-muted-foreground opacity-60 group-hover:opacity-100"
            />
          </Link>
        ) : (
          <span className="truncate text-sm font-medium">{row.name}</span>
        )}
        <StateBadge kind={badgeKind} value={row.state} className="shrink-0" />
      </div>
      {row.image && (
        <p title={row.image} className="mt-1 truncate font-mono text-xs text-muted-foreground">
          <span className="sr-only">Image </span>
          {row.image}
        </p>
      )}
      <div className="flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted-foreground not-empty:mt-1.5">
        {row.platformName && (
          <span className={metaClass}>
            <Server aria-hidden="true" />
            <span className="sr-only">Platform </span>
            <span className="truncate" title={row.platformName}>
              {row.platformName}
            </span>
          </span>
        )}
        {row.releaseVersion && (
          <span className={metaClass}>
            <Layers aria-hidden="true" />
            <span className="truncate" title={row.releaseVersion}>
              Release {row.releaseVersion}
            </span>
          </span>
        )}
        {row.createdAt && (
          <span className={metaClass}>
            <CalendarClock aria-hidden="true" />
            {kind === 'Stack' ? 'Release created' : 'Created'}
            <Timestamp value={row.createdAt} formatDateTime={format} className="text-xs text-muted-foreground" />
          </span>
        )}
        {!!row.publishedPorts?.length && (
          <span className={metaClass}>
            <EthernetPort aria-hidden="true" />
            <span className="sr-only">Published ports </span>
            <span>{row.publishedPorts.join(', ')}</span>
          </span>
        )}
      </div>
    </li>
  );
}

export function FleetMembers({
  title,
  kinds,
  state,
  tags,
}: {
  title: string;
  kinds?: string;
  state?: string;
  tags: string[];
}) {
  const [{ page, search }, setFilter] = useState({ page: 1, search: '' });
  const searchMembers = useCallback((value: string) => {
    const term = value.trim();
    setFilter((current) => (current.search === term ? current : { page: 1, search: term }));
  }, []);
  const read = useDashboardRead(
    'getDashboardFleet',
    {
      query: {
        read: 'Members',
        kinds,
        state,
        tags,
        search: search || undefined,
        limit: pageSize,
        offset: (page - 1) * pageSize,
      },
    },
    { retry: false },
  );
  const result = !read.isError ? read.data?.data : undefined;
  const total = result?.total;
  const pages = total === undefined ? 0 : Math.max(page, Math.ceil(total / pageSize));
  const indicator = kinds === 'Container' ? 'container' : kinds === 'Platform' ? 'platform' : undefined;
  return (
    <>
      <SheetHeader className="shrink-0 border-b pe-10">
        <SheetTitle className="flex items-center gap-2">
          {state && <StateIndicator kind={indicator} value={state} />}
          {title}
        </SheetTitle>
        <SheetDescription aria-live="polite">
          {total !== undefined
            ? `${total.toLocaleString()} ${total === 1 ? 'match' : 'matches'}`
            : 'Resources in this view'}
        </SheetDescription>
      </SheetHeader>
      <div className="shrink-0 px-4">
        <SearchField
          placeholder="Search names, images or platforms…"
          ariaLabel="Search by name, image or platform"
          maxLength={128}
          onSearch={searchMembers}
        />
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto border-t">
        {read.isError ? (
          <div className="p-4">
            <ResourceReadError error={read.error} refetch={read.refetch} isFetching={read.isFetching} />
          </div>
        ) : !result ? (
          <Loader />
        ) : result.rows.length ? (
          <ul className="divide-y">
            {result.rows.map((row) => (
              <MemberRow key={row.kind + row.id} row={row} />
            ))}
          </ul>
        ) : (
          <p className="p-4 text-sm text-muted-foreground">
            {search ? 'No resources match your search.' : 'No resources in this view.'}
          </p>
        )}
      </div>
      {pages > 1 && (
        <SheetFooter className="shrink-0 border-t py-2">
          <PaginationControls
            currentPage={page}
            totalPages={pages}
            onPageChange={(page) => {
              if (page >= 1 && page <= pages) setFilter({ page, search });
            }}
          />
        </SheetFooter>
      )}
    </>
  );
}
