import { canShowCachedResource } from '@/lib/request-error';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { ResourceOverview } from '@/components/custom/resource-overview';
import { AppContent } from '@/components/custom/app-content';
import { ResourceType } from '@/api/types';
import TaskSheet from '@/components/custom/task-sheet';
import { useSelectedResources } from '@/lib/atoms';
import { useMemo, useState, useEffect } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router';
import { RegularResourceComponents } from './types';
import { ResourceHeader } from './resource-header';
import { ResourceOverviewSkeleton } from './resource-skeleton';

type RegularResourceViewProps<T = any> = {
  Components: RegularResourceComponents<T>;
  type: string;
  showTaskSheet?: boolean;
};

const EMPTY_ITEMS: never[] = [];

export const RegularResourceView = <T,>({ Components, type, showTaskSheet = true }: RegularResourceViewProps<T>) => {
  const navigate = useNavigate();
  const platformId = useParams().platformId ?? '';
  const [searchParams, setSearchParams] = useSearchParams();
  const [search, setSearch] = useState('');
  const [addDialogOpen, setAddDialogOpen] = useState(false);

  const { items, capabilities, isLoading = false, error, refetch, isFetching } = Components.useData?.(platformId) ?? {};
  const headerCfg = Components.header ?? { showSearch: true, showAdd: true };
  const AddDialog = headerCfg.AddDialog;

  const filtered = useMemo(
    () => (Components.filterItems ? Components.filterItems(items ?? EMPTY_ITEMS, search) : (items ?? EMPTY_ITEMS)),
    [items, search, Components],
  );

  const overview = Components.overview;
  const requestedFilter =
    searchParams.get('updates') === 'available' && overview?.filters.some((filter) => filter.id === 'updates')
      ? 'updates'
      : searchParams.get('overview');
  const activeFilter = overview?.filters.find((filter) => filter.id === requestedFilter);
  const activeId = activeFilter?.id ?? 'all';
  const visibleItems = useMemo(
    () => (activeFilter?.matches ? filtered.filter(activeFilter.matches) : filtered),
    [filtered, activeFilter],
  );
  const selectOverview = (id: string) => {
    const nextId = id === activeId ? 'all' : id;
    setSearchParams((current) => {
      const next = new URLSearchParams(current);
      next.delete('overview');
      if (overview?.filters.some((filter) => filter.id === 'updates')) next.delete('updates');
      if (nextId === 'updates') next.set('updates', 'available');
      else if (nextId !== 'all') next.set('overview', nextId);
      return next;
    });
  };

  const [_, setSelected] = useSelectedResources(type as ResourceType);
  useEffect(() => {
    return () => setSelected([]);
  }, [type, setSelected, activeId]);

  const hasActiveUrlFilters =
    (headerCfg.showTagFilter && searchParams.getAll('tags').some((tag) => tag.trim().length > 0)) ||
    (headerCfg.showPlatformFilter && Boolean(searchParams.get('platformId')?.trim())) ||
    (headerCfg.activeFilterParams?.some((parameter) => Boolean(searchParams.get(parameter)?.trim())) ?? false);

  const Content = Components.Content;
  const hasItems = Array.isArray(items)
    ? items.length > 0
    : Boolean((items as { items?: unknown[] } | undefined)?.items?.length);
  const showContent = !error || (hasItems && canShowCachedResource(error));

  return (
    <div className="flex-col justify-between relative">
      <AppContent className="flex flex-col gap-(--section-gap)">
        <ResourceHeader
          type={type}
          icon={Components.Icon}
          title={headerCfg.title}
          subtitle={headerCfg.subtitle}
          showSearch={headerCfg.showSearch}
          showAdd={headerCfg.showAdd}
          showTagFilter={headerCfg.showTagFilter}
          showPlatformFilter={headerCfg.showPlatformFilter}
          addDisabled={!capabilities?.canWrite}
          addButtonTitle={headerCfg.addButtonTitle}
          Extra={headerCfg.Extra}
          onSearch={setSearch}
          onAdd={() => (AddDialog ? setAddDialogOpen(true) : navigate(headerCfg.addButtonUrl ?? './add'))}
        />
        {AddDialog && <AddDialog open={addDialogOpen} onOpenChange={setAddDialogOpen} />}

        {Components.SubHeader && <Components.SubHeader />}

        {overview && isLoading && !hasItems && showContent && (
          <ResourceOverviewSkeleton count={overview.filters.length} />
        )}
        {overview &&
          (hasItems || hasActiveUrlFilters || activeId !== 'all') &&
          (!isLoading || hasItems) &&
          showContent && (
            <ResourceOverview
              label={overview.label}
              activeId={activeId}
              onSelect={selectOverview}
              metrics={overview.filters.map(({ matches, ...filter }) => ({
                ...filter,
                value: matches ? filtered.filter(matches).length : filtered.length,
              }))}
            />
          )}

        {!!error && (
          <ResourceReadError error={error} refetch={refetch} isFetching={isFetching} stale={hasItems && showContent} />
        )}
        {showContent && (
          <Content
            key={Components.GroupActions ? activeId : undefined}
            items={visibleItems}
            actions={Components.DropdownActions ?? {}}
            isLoading={isLoading}
            isFiltered={Boolean(search.trim()) || hasActiveUrlFilters || activeId !== 'all'}
          />
        )}
      </AppContent>

      {!error && Components.GroupActions && (
        <Components.GroupActions items={overview ? visibleItems : (items ?? EMPTY_ITEMS)} />
      )}
      {showTaskSheet && type !== 'Alert' && type !== 'Activity' && <TaskSheet type={type as ResourceType} />}
    </div>
  );
};
