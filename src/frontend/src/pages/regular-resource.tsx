import { canShowCachedResource } from '@/lib/request-error';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { AppContent } from '@/components/custom/app-content';
import { ResourceType } from '@/api/types';
import TaskSheet from '@/components/custom/task-sheet';
import { useSelectedResources } from '@/lib/atoms';
import { useMemo, useState, useEffect } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router';
import { RegularResourceComponents } from './types';
import { ResourceHeader } from './resource-header';

type RegularResourceViewProps<T = any> = {
  Components: RegularResourceComponents<T>;
  type: string;
  showTaskSheet?: boolean;
};

const EMPTY_ITEMS: never[] = [];

export const RegularResourceView = <T,>({ Components, type, showTaskSheet = true }: RegularResourceViewProps<T>) => {
  const navigate = useNavigate();
  const platformId = useParams().platformId ?? '';
  const [searchParams] = useSearchParams();
  const updatesFilter = searchParams.get('updates') === 'available';
  const [search, setSearch] = useState('');
  const [addDialogOpen, setAddDialogOpen] = useState(false);

  const { items, capabilities, isLoading = false, error, refetch, isFetching } = Components.useData?.(platformId) ?? {};
  const headerCfg = Components.header ?? { showSearch: true, showAdd: true };
  const AddDialog = headerCfg.AddDialog;

  const filtered = useMemo(
    () => (Components.filterItems ? Components.filterItems(items ?? EMPTY_ITEMS, search) : (items ?? EMPTY_ITEMS)),
    [items, search, Components],
  );

  const [_, setSelected] = useSelectedResources(type as ResourceType);
  useEffect(() => {
    return () => setSelected([]);
  }, [type, setSelected, updatesFilter]);

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

        {!!error && (
          <ResourceReadError error={error} refetch={refetch} isFetching={isFetching} stale={hasItems && showContent} />
        )}
        {showContent && (
          <Content
            key={Components.GroupActions ? String(updatesFilter) : undefined}
            items={filtered}
            actions={Components.DropdownActions ?? {}}
            isLoading={isLoading}
            isFiltered={Boolean(search.trim()) || hasActiveUrlFilters}
          />
        )}
      </AppContent>

      {!error && Components.GroupActions && <Components.GroupActions items={filtered} />}
      {showTaskSheet && type !== 'Alert' && type !== 'Activity' && <TaskSheet type={type as ResourceType} />}
    </div>
  );
};
