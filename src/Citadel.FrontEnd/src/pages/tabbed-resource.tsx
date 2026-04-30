import { ResourceType } from '@/api/types';
import { SearchField } from '@/components/custom/search-field';
import TaskSheet from '@/components/custom/task-sheet';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useSelectedResources } from '@/lib/atoms';
import { useLocalStorage } from '@/lib/hooks';
import { cn } from '@/lib/utils';
import { Plus } from 'lucide-react';
import { useMemo, useState, useEffect } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router';
import { ResourceHeader } from './resource-header';
import { TabbedResourceComponents } from './types';

type TabbedResourceViewProps<T = any> = {
  Components: TabbedResourceComponents<T>;
  type: ResourceType;
};

export const TabbedResourceView = <T,>({ Components, type }: TabbedResourceViewProps<T>) => {
  const navigate = useNavigate();
  const location = useLocation();
  const { platformId = '' } = useParams();
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

  const tabs = Components.Tabs;
  const routeSegment = location.pathname.split('/').filter(Boolean)[0];
  const routeTabDef = tabs.find((t) => t.slug === routeSegment);
  const [storedActiveTab, setStoredActiveTab] = useLocalStorage(`${type}-tabs-active`, tabs[0]?.label ?? '');
  const storedTabDef = tabs.find((t) => t.label === storedActiveTab);
  const activeTab = routeTabDef?.label ?? storedTabDef?.label ?? tabs[0]?.label ?? '';
  const activeTabDef = tabs.find((t) => t.label === activeTab) ?? tabs[0];
  const showTopSearch = headerCfg.showSearch && !(activeTabDef?.Header?.showSearch ?? false);

  const navigateToTab = (tab: (typeof tabs)[number] | undefined) => {
    if (tab?.slug) {
      navigate(`/${tab.slug}`);
    }
  };

  const handleTabChange = (nextTab: string) => {
    setStoredActiveTab(nextTab);
    navigateToTab(tabs.find((t) => t.label === nextTab));
  };

  return (
    <div className="flex-col justify-between relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="w-full rounded-lg border-border bg-background p-4 flex flex-col gap-4">
          <ResourceHeader
            type={type}
            icon={Components.Icon}
            title={headerCfg.title}
            subtitle={headerCfg.subtitle}
            showSearch={showTopSearch}
            showAdd={headerCfg.showAdd}
            addButtonTitle={headerCfg.addButtonTitle}
            Extra={headerCfg.Extra}
            onSearch={setSearch}
            onAdd={() => navigate('./add')}
          />

          {Components.SubHeader && <Components.SubHeader />}

          <Tabs value={activeTab} onValueChange={handleTabChange} className="gap-4">
            <div className="flex sm:flex-row flex-col gap-2 items-center justify-between">
              <div className="flex items-center gap-2">
                <TabsList className={cn('overflow-x-auto')}>
                  {tabs.map((tab) => (
                    <TabsTrigger
                      key={tab.label}
                      value={tab.label}
                      className="text-xs"
                      onClick={() => navigateToTab(tab)}
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
        </div>
      </div>

      {Components.GroupActions && <Components.GroupActions items={items ?? []} />}
      <TaskSheet type={type} />
    </div>
  );
};
