import { ResourceType } from '@/api/types';
import TaskSheet from '@/components/custom/task-sheet';
import { useSelectedResources } from '@/lib/atoms';
import { useMemo, useState, useEffect } from 'react';
import { useNavigate, useParams } from 'react-router';
import { RegularResourceComponents } from './types';
import { ResourceHeader } from './resource-header';

type RegularResourceViewProps<T = any> = {
  Components: RegularResourceComponents<T>;
  type: ResourceType;
};

export const RegularResourceView = <T,>({ Components, type }: RegularResourceViewProps<T>) => {
  const navigate = useNavigate();
  const platformId = useParams().platformId ?? '';
  const [search, setSearch] = useState('');

  const { items, isLoading = false } = Components.useData?.(platformId) ?? {};
  const headerCfg = Components.header ?? { showSearch: true, showAdd: true };

  const filtered = useMemo(
    () => (Components.filterItems ? Components.filterItems(items ?? [], search) : items),
    [items, search, Components],
  );

  const [_, setSelected] = useSelectedResources(type);
  useEffect(() => {
    return () => setSelected([]);
  }, [type, setSelected]);

  const Content = Components.Content;

  return (
    <div className="flex-col justify-between relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="w-full rounded-lg border-border bg-background p-4 flex flex-col gap-4">
          <ResourceHeader
            type={type}
            icon={Components.Icon}
            title={headerCfg.title}
            subtitle={headerCfg.subtitle}
            showSearch={headerCfg.showSearch}
            showAdd={headerCfg.showAdd}
            addButtonTitle={headerCfg.addButtonTitle}
            Extra={headerCfg.Extra}
            onSearch={setSearch}
            onAdd={() => navigate('./add')}
          />

          {Components.SubHeader && <Components.SubHeader />}

          <Content
            items={filtered ?? []}
            actions={Components.DropdownActions ?? {}}
            isLoading={isLoading}
            isFiltered={Boolean(search.trim())}
          />
        </div>
      </div>

      {Components.GroupActions && <Components.GroupActions items={items ?? []} />}
      <TaskSheet type={type} />
    </div>
  );
};
