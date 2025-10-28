import { useDockerResourceParamType, useLocalStorage } from '@/lib/hooks';
import { useNavigate, useParams } from 'react-router';
import { RequiredDockerComponents } from './types';
import { DockerResourceType, PluralResourceMap } from '@/api/types';
import { SearchField } from '@/components/ui/SearchField';
import { Button } from '@/components/ui/button';
import { Plus } from 'lucide-react';
import NotFound from './NotFound';
import { useMemo, useState } from 'react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { DockerResourceComponents } from '@/features/docker-resources';
import TaskSheet from '@/components/custom/task-sheet';

const DockerResourcePage = () => {
  const type = useDockerResourceParamType()!;

  const Components = DockerResourceComponents[type];
  if (!Components) return <NotFound />;

  return <ResourceView key={type} Components={Components} type={type} />;
};

const ResourceView = <T,>({ Components, type }: ResourceViewProps<T>) => {
  const navigate = useNavigate();
  const platformId = useParams().platformId ?? '';
  const [search, setSearch] = useState('');
  const tabs = Components.tabs;

  const [activeTab, setActiveTab] = useLocalStorage(`${useParams().type}.active-tab`, tabs?.[0]?.label ?? 'default');

  const allTabData = tabs?.map((tab) => tab.useData?.(platformId));
  const singleData = Components.useData?.(platformId);

  const currentTabIndex = tabs?.findIndex((t) => t.label === activeTab);
  const currentData =
    tabs && currentTabIndex !== undefined && currentTabIndex >= 0 ? allTabData?.[currentTabIndex] : singleData;

  const headerCfg = Components.tabs?.[currentTabIndex ?? 0]?.header ??
    Components.header ?? { showSearch: true, showAdd: true };

  const { items = [], isLoading = false } = currentData ?? {};
  const filtered = useMemo(
    () => (Components.filterItems ? Components.filterItems(items, search) : items),
    [items, search, Components],
  );

  const ActiveContent =
    tabs && currentTabIndex !== undefined && currentTabIndex >= 0 ? tabs[currentTabIndex].Content : Components.Table!;

  return (
    <div className="flex-col justify-between relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="w-full rounded-lg border-border bg-background p-4">
          {/* Header */}
          <div className="sm:flex sm:justify-between">
            <div className="mb-3 flex items-baseline gap-1">
              <div className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                {Components.Icon}
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
                  className="inline-flex items-center bg-primary hover:bg-primary/80 font-medium rounded-sm text-xs px-2.5 py-2.5">
                  <Plus className="h-3 w-3" /> Add {type}
                </Button>
              )}
              {headerCfg.Extra && <headerCfg.Extra />}
            </div>
          </div>

          {/* Tabs */}
          {tabs ? (
            <Tabs value={activeTab} onValueChange={setActiveTab}>
              <TabsList className="mb-2">
                {tabs.map((tab) => (
                  <TabsTrigger key={tab.label} value={tab.label}>
                    {tab.label}
                  </TabsTrigger>
                ))}
              </TabsList>

              {tabs.map((tab, i) => (
                <TabsContent key={tab.label} value={tab.label}>
                  <div className="space-y-1 rounded-sm border p-1 shadow-xs">
                    <tab.Content
                      items={
                        Components.filterItems
                          ? Components.filterItems(allTabData?.[i]?.items ?? [], search)
                          : (allTabData?.[i]?.items ?? [])
                      }
                      actions={tab.DropdownActions ?? {}}
                      isLoading={allTabData?.[i]?.isLoading ?? false}
                    />
                  </div>
                  {tab.GroupActions && <tab.GroupActions items={items} />}
                </TabsContent>
              ))}
            </Tabs>
          ) : (
            <div className="space-y-1 rounded-sm border p-1 shadow-xs">
              <ActiveContent items={filtered} isLoading={isLoading} actions={Components.DropdownActions ?? {}} />
            </div>
          )}
        </div>
      </div>

      {Components.GroupActions && <Components.GroupActions items={items} />}
      <TaskSheet type={type} />
    </div>
  );
};

type ResourceViewProps<T = any> = {
  Components: RequiredDockerComponents<T>;
  type: DockerResourceType;
};

export default DockerResourcePage;
