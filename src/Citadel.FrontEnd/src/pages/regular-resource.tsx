import { ResourceType } from '@/api/types';
import TaskSheet from '@/components/custom/task-sheet';
import { useSelectedResources } from '@/lib/atoms';
import { useMemo, useState, useEffect } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router';
import { RegularResourceComponents } from './types';
import { ResourceHeader } from './resource-header';

type RegularResourceViewProps<T = any> = {
  Components: RegularResourceComponents<T>;
  type: ResourceType;
};

const EMPTY_ITEMS: never[] = [];

export const RegularResourceView = <T,>({ Components, type }: RegularResourceViewProps<T>) => {
  const navigate = useNavigate();
  const platformId = useParams().platformId ?? '';
  const [searchParams] = useSearchParams();
  const [search, setSearch] = useState('');
  const [addDialogOpen, setAddDialogOpen] = useState(false);

  const { items, capabilities, isLoading = false } = Components.useData?.(platformId) ?? {};
  const headerCfg = Components.header ?? { showSearch: true, showAdd: true };
  const AddDialog = headerCfg.AddDialog;

  const filtered = useMemo(
    () => (Components.filterItems ? Components.filterItems(items ?? EMPTY_ITEMS, search) : (items ?? EMPTY_ITEMS)),
    [items, search, Components],
  );

  const [_, setSelected] = useSelectedResources(type);
  useEffect(() => {
    return () => setSelected([]);
  }, [type, setSelected]);

  const hasActiveUrlFilters =
    (headerCfg.showTagFilter && searchParams.getAll('tags').some((tag) => tag.trim().length > 0)) ||
    (headerCfg.showPlatformFilter && Boolean(searchParams.get('platformId')?.trim())) ||
    (headerCfg.activeFilterParams?.some((parameter) => Boolean(searchParams.get(parameter)?.trim())) ?? false);

  const Content = Components.Content;

  return (
    <div className="flex-col justify-between relative">
      <div className="mx-auto w-full max-w-[1440px] px-4 py-4 sm:px-6">
        <div className="w-full rounded-lg border-border bg-background p-4 flex flex-col gap-4">
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

          <Content
            items={filtered}
            actions={Components.DropdownActions ?? {}}
            isLoading={isLoading}
            isFiltered={Boolean(search.trim()) || hasActiveUrlFilters}
          />
        </div>
      </div>

      {Components.GroupActions && <Components.GroupActions items={items ?? EMPTY_ITEMS} />}
      <TaskSheet type={type} />
    </div>
  );
};
