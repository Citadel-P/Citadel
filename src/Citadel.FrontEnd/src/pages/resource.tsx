import { useResourceParamType, useLocalStorage } from '@/lib/hooks';
import { useNavigate, useParams } from 'react-router';
import { RequiredComponents } from './types';
import { PluralResourceMap, ResourceType } from '@/api/types';
import { SearchField } from '@/components/custom/search-field';
import { Button } from '@/components/ui/button';
import { Plus } from 'lucide-react';
import NotFound from './not-found';
import { useMemo, useState, useEffect } from 'react';
import { ResourceComponents } from '@/features';
import TaskSheet from '@/components/custom/task-sheet';
import { useSelectedResources } from '@/lib/atoms';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { cn } from '@/lib/utils';

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

  const { items, isLoading = false } = Components.useData?.(platformId) ?? {};

  const headerCfg = Components.header ?? { showSearch: true, showAdd: true };

  const filtered = useMemo(
    () => (Components.filterItems ? Components.filterItems(items ?? [], search) : items),
    [items, search, Components],
  );

  const ActiveContent = Components.Content!;
  const Icon = Components.Icon;

  const [_, setSelected] = useSelectedResources(type);
  useEffect(() => {
    return () => setSelected([]);
  }, [type, setSelected]);

  const hasTabs = Array.isArray(Components.Tabs) && Components.Tabs.length > 0;
  const tabs = Components.Tabs ?? [];
  const [activeTab, setActiveTab] = useLocalStorage(`${type}-tabs-active`, tabs[0]?.label ?? '');
  const activeTabDef = tabs.find((t) => t.label === activeTab) ?? tabs[0];
  const showTopSearch = headerCfg.showSearch && !(hasTabs && (activeTabDef?.Header?.showSearch ?? false));

  return (
    <div className="flex-col justify-between relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="w-full rounded-lg border-border bg-background p-4 flex flex-col gap-4">
          {/* Header */}
          <div className="flex flex-col sm:flex-row gap-2 sm:justify-between">
            <div className="flex items-center gap-3">
              <div className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                {Icon}
                <span className="sr-only">{PluralResourceMap[type]}</span>
              </div>
              <div className="flex flex-col">
                <div className="text-md font-bold text-foreground">{headerCfg.title ?? PluralResourceMap[type]}</div>
                <p className="text-xs text-muted-foreground truncate">{headerCfg.subtitle}</p>
              </div>
            </div>
            <div className="flex gap-2">
              {showTopSearch && <SearchField onSearch={setSearch} />}
              {headerCfg.showAdd && (
                <Button
                  type="button"
                  onClick={() => navigate('./add')}
                  className="inline-flex items-center bg-primary hover:bg-primary/80 rounded-sm text-sm px-2.5 py-2.5">
                  <Plus className="h-3 w-3" /> {headerCfg.addButtonTitle ?? `Add ${type}`}
                </Button>
              )}
              {headerCfg.Extra && <headerCfg.Extra />}
            </div>
          </div>

          {/* Sub Header */}
          {Components.SubHeader && <Components.SubHeader />}

          {/* Content */}
          {Components.Content ? (
            <ActiveContent
              items={filtered ?? []}
              actions={Components.DropdownActions ?? {}}
              isLoading={isLoading}
              isFiltered={Boolean(search.trim())}
            />
          ) : hasTabs ? (
            <Tabs value={activeTab} onValueChange={setActiveTab} className="gap-4">
              <div className="flex sm:flex-row flex-col gap-2 items-center justify-between">
                <div className="flex  items-center gap-2">

                  <TabsList className={cn('overflow-x-auto')}>
                    {tabs.map((tab) => (
                      <TabsTrigger
                        key={tab.label}
                        value={tab.label}
                        className="text-xs"
                        disabled={tab.disabled?.(null as any) ?? false}>
                        {tab.label}
                      </TabsTrigger>
                    ))}
                  </TabsList>
                </div>

                <div className="flex flex-col sm:flex-row gap-1">
                  {(activeTabDef?.Header?.showSearch ?? false) && (
                    <SearchField
                      onSearch={(q) => {
                        activeTabDef?.Header?.onSearch?.(q);
                        setSearch(q);
                      }}
                      placeholder={activeTabDef?.Header?.searchPlaceholder}
                    />
                  )}
                  {(activeTabDef?.Header?.showAdd ?? headerCfg.showAdd) && (
                    <Button
                      type="button"
                      onClick={() => {
                        const target = activeTabDef?.Header?.addButtonUrl ?? './add';
                        navigate(target);
                      }}
                      className="inline-flex items-center bg-primary hover:bg-primary/80 rounded-sm text-sm px-2.5 py-2.5">
                      <Plus className="h-3 w-3" />{' '}
                      {activeTabDef?.Header?.addButtonTitle ?? headerCfg.addButtonTitle ?? `Add ${type}`}
                    </Button>
                  )}
                  {activeTabDef?.Header?.Extra && <activeTabDef.Header.Extra />}
                </div>
              </div>

              {tabs.map((tab) => (
                <TabsContent key={tab.label} value={tab.label}>
                  <tab.Content
                    items={filtered ?? []}
                    actions={Components.DropdownActions ?? {}}
                    isLoading={isLoading}
                    isFiltered={Boolean(search.trim())}
                  />
                </TabsContent>
              ))}
            </Tabs>
          ) : (
            <></>
          )}
        </div>
      </div>

      {Components.GroupActions && <Components.GroupActions items={items ?? []} />}
      <TaskSheet type={type} />
    </div>
  );
};

type ResourceViewProps<T = any> = {
  Components: RequiredComponents<T>;
  type: ResourceType;
};

export default ResourcePage;
