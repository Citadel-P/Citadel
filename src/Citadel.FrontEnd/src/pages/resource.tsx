import { useResourceParamType } from '@/lib/hooks';
import { useNavigate, useParams } from 'react-router';
import { RequiredComponents } from './types';
import { PluralResourceMap, ResourceType } from '@/api/types';
import { SearchField } from '@/components/custom/search-field';
import { Button } from '@/components/ui/button';
import { Plus } from 'lucide-react';
import NotFound from './not-found';
import { useMemo, useState } from 'react';
import { ResourceComponents } from '@/features';
import TaskSheet from '@/components/custom/task-sheet';

const ResourcePage = () => {
  const type = useResourceParamType()!;

  const Components = ResourceComponents[type];
  if (!Components) return <NotFound />;

  return <ResourceView key={type} Components={Components} type={type} />;
};

const ResourceView = <T,>({ Components, type }: ResourceViewProps<T>) => {
  const navigate = useNavigate();
  const platformId = useParams().platformId ?? '';
  const [search, setSearch] = useState('');

  const { items = [], isLoading = false } = Components.useData?.(platformId) ?? {};

  const headerCfg = Components.header ?? { showSearch: true, showAdd: true };

  const filtered = useMemo(
    () => (Components.filterItems ? Components.filterItems(items, search) : items),
    [items, search, Components],
  );

  const ActiveContent = Components.Content!;
  const Icon = Components.Icon;

  return (
    <div className="flex-col justify-between relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="w-full rounded-lg border-border bg-background p-4 flex flex-col gap-3">
          {/* Header */}
          <div className="sm:flex sm:justify-between">
            <div className="flex items-center gap-1">
              <div className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                {Icon}
                <span className="sr-only">{PluralResourceMap[type]}</span>
              </div>
              <div className="text-md font-bold text-foreground">{PluralResourceMap[type]}</div>
            </div>
            <div className="flex gap-2">
              {headerCfg.showSearch && <SearchField onSearch={setSearch} />}
              {headerCfg.showAdd && (
                <Button
                  type="button"
                  onClick={() => navigate('./add')}
                  className="inline-flex items-center bg-primary hover:bg-primary/80 rounded-sm text-sm px-2.5 py-2.5">
                  <Plus className="h-3 w-3" /> Add {type}
                </Button>
              )}
              {headerCfg.Extra && <headerCfg.Extra />}
            </div>
          </div>

          {/* Sub Header */}
          {Components.SubHeader && <Components.SubHeader />}

          {/* Table */}
          <div className="rounded-sm border p-1 shadow-xs">
            <ActiveContent
              items={filtered}
              actions={Components.DropdownActions ?? {}}
              isLoading={isLoading}
              isFiltered={Boolean(search.trim())}
            />
          </div>
        </div>
      </div>

      {Components.GroupActions && <Components.GroupActions items={items} />}
      <TaskSheet type={type} />
    </div>
  );
};

type ResourceViewProps<T = any> = {
  Components: RequiredComponents<T>;
  type: ResourceType;
};

export default ResourcePage;
