import { PluralResourceMap, ResourceType } from '@/api/types';
import { PageContainer } from '@/components/custom/common';
import { SearchField } from '@/components/custom/search-field';
import TaskSheet from '@/components/custom/task-sheet';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useSelectedResources } from '@/lib/atoms';
import { useLocalStorage } from '@/lib/hooks';
import { Plus } from 'lucide-react';
import { useState, useEffect } from 'react';
import { useNavigate, useParams } from 'react-router';
import { ResourceHeader } from './resource-header';
import { TabElement, TabbedResourceComponents } from './types';
import { useAppContext } from '@/lib/context/app-context';
import { hasCapability } from '@/lib/resource-capabilities';

type TabbedResourceViewProps<T = any> = {
  Components: TabbedResourceComponents<T>;
  type: ResourceType;
  tab?: ResourceType;
};

export const TabbedResourceView = <T,>({ Components, type, tab }: TabbedResourceViewProps<T>) => {
  const navigate = useNavigate();
  const { platformId = '' } = useParams();
  const [search, setSearch] = useState('');
  const [_, setSelected] = useSelectedResources(type);

  const tabs = Components.Tabs;
  const header = {
    showSearch: true,
    showAdd: true,
    ...Components.header,
  };

  const [storedTab, setStoredTab] = useLocalStorage(`${type}-active-tab`, tabs[0].label);

  const routeTab =
    tab && PluralResourceMap[tab]
      ? tabs.find((t) => {
          const plural = PluralResourceMap[tab].toLowerCase();
          return t.label.toLowerCase() === plural || t.slug?.toLowerCase() === plural;
        })
      : undefined;

  const activeTab = routeTab ?? tabs.find((t) => t.label === storedTab) ?? tabs[0];

  useEffect(() => {
    return () => setSelected([]);
  }, [type, setSelected]);

  const basePath = platformId ? `/platforms/${platformId}/${type.toLocaleLowerCase()}` : `/${type.toLowerCase()}`;

  const navigateToTab = (t: (typeof tabs)[number]) => {
    if (!t.slug) return;
    navigate(`${basePath}/${t.slug}`);
  };

  const handleTabChange = (label: string) => {
    setStoredTab(label);
    const t = tabs.find((t) => t.label === label);
    if (t) navigateToTab(t);
  };

  // Header
  const tabHeader = activeTab.Header ?? {};

  const showTopSearch = header.showSearch && !tabHeader.showSearch;

  const showTabSearch = tabHeader.showSearch;
  const showAdd = tabHeader.showAdd ?? header.showAdd;
  const { currentPlatform } = useAppContext();
  const addDisabled = !hasCapability(currentPlatform, 'canWrite');

  return (
    <div className="flex-col relative">
      <PageContainer className="flex flex-col gap-4">
        <ResourceHeader
          type={type}
          icon={Components.Icon}
          title={header.title}
          subtitle={header.subtitle}
          showSearch={showTopSearch}
          showAdd={header.showAdd}
          showTagFilter={header.showTagFilter}
          showPlatformFilter={header.showPlatformFilter}
          addDisabled={addDisabled}
          addButtonTitle={header.addButtonTitle}
          Extra={header.Extra}
          onSearch={setSearch}
          onAdd={() => navigate('./add')}
        />

        {Components.SubHeader && <Components.SubHeader />}

        <Tabs value={activeTab.label} onValueChange={handleTabChange}>
          <div className="flex flex-col sm:flex-row justify-between gap-2">
            {/* Tabs */}
            <TabsList className="overflow-x-auto">
              {tabs.map((t) => (
                <TabsTrigger
                  key={t.label}
                  value={t.label}
                  disabled={t.disabled?.(null as any) ?? false}
                  onClick={() => navigateToTab(t)}>
                  {t.Label ? <t.Label /> : t.label}
                </TabsTrigger>
              ))}
            </TabsList>

            {/* Actions */}
            <div className="flex gap-1 flex-col sm:flex-row">
              {showTabSearch && (
                <SearchField
                  placeholder={tabHeader.searchPlaceholder}
                  onSearch={(q) => {
                    tabHeader.onSearch?.(q);
                    setSearch(q);
                  }}
                />
              )}

              {showAdd && (
                <Button
                  onClick={() => navigate(tabHeader.addButtonUrl ?? './add')}
                  disabled={addDisabled}
                  className="bg-primary hover:bg-primary/80 text-sm px-2.5 py-2.5">
                  <Plus className="h-3 w-3" />
                  {tabHeader.addButtonTitle ?? header.addButtonTitle ?? `Add ${type}`}
                </Button>
              )}

              {tabHeader.Extra && <tabHeader.Extra />}
            </div>
          </div>

          {/* Content */}
          {tabs.map((t) => (
            <TabsContent key={t.label} value={t.label}>
              <TabContentWrapper key={t.label} tab={t} Components={Components} search={search} />
            </TabsContent>
          ))}
        </Tabs>
      </PageContainer>

      <TaskSheet type={type} />
    </div>
  );
};

type TabContentWrapperProps<T> = {
  tab: TabElement<T>;
  Components: TabbedResourceComponents<T>;
  search: string;
};

const TabContentWrapper = <T,>({ tab, Components, search }: TabContentWrapperProps<T>) => {
  const { items = [], isLoading = false } = tab.useData?.() ?? {};
  const filtered = Components.filterItems ? Components.filterItems(items, search) : items;
  const GroupActions = tab.GroupActions ?? Components.GroupActions;
  return (
    <>
      <tab.Content
        items={filtered}
        actions={tab.DropdownActions ?? Components.DropdownActions ?? {}}
        isLoading={isLoading}
        isFiltered={!!search.trim()}
      />
      {GroupActions && <GroupActions items={items} />}
    </>
  );
};
