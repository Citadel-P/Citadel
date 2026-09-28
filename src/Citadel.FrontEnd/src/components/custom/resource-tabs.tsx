import { useLocalStorage, useStickySentinel } from '@/lib/hooks';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/tabs';
import { cn } from '@/lib/utils';
import { RequiredFormFields, ResourceTabElement } from '@/pages/types';
import { useSegmentTitle } from '@/lib/atoms';
import { useEffect, useMemo } from 'react';
import { useLocation, useNavigate } from 'react-router';

const getTabHash = (label: string) => label.toLowerCase().replace(/\s+/g, '-');

export const ResourceTabs = ({
  localKey,
  resource,
  tabs,
  metadataChanged,
}: {
  localKey: string;
  resource: RequiredFormFields;
  tabs: ResourceTabElement<any>[];
  metadataChanged?: boolean;
}) => {
  const [_, setSegmentTitle] = useSegmentTitle();
  const location = useLocation();
  const navigate = useNavigate();
  const firstTabLabel = tabs[0]?.label;

  useEffect(() => {
    if (resource?.name) {
      setSegmentTitle({ action: firstTabLabel, name: resource?.name });
    }
  }, [firstTabLabel, resource?.name, setSegmentTitle]);

  const [activeTab, setActiveTab] = useLocalStorage(localKey, tabs[0]?.label ?? 'default');
  const { sentinelRef, isStuck } = useStickySentinel();
  const enabledTabs = useMemo(() => tabs.filter((tab) => !(tab.disabled?.(resource) ?? false)), [tabs, resource]);
  const hashTab = useMemo(() => {
    const hash = (location.hash || window.location.hash).replace(/^#/, '');
    if (!hash) return undefined;

    return tabs.find((tab) => getTabHash(tab.label) === hash && !(tab.disabled?.(resource) ?? false));
  }, [location.hash, resource, tabs]);
  const currentTabEnabled = useMemo(
    () => tabs.some((tab) => tab.label === activeTab && !(tab.disabled?.(resource) ?? false)),
    [activeTab, resource, tabs],
  );
  const effectiveActiveTab = hashTab?.label ?? (currentTabEnabled ? activeTab : (enabledTabs[0]?.label ?? activeTab));

  useEffect(() => {
    if (effectiveActiveTab !== activeTab) {
      setActiveTab(effectiveActiveTab);
    }
  }, [activeTab, effectiveActiveTab, setActiveTab]);

  const handleTabChange = (tab: string) => {
    setActiveTab(tab);
    navigate(`${location.pathname}${location.search}#${getTabHash(tab)}`, { replace: true });
  };

  return (
    <Tabs value={effectiveActiveTab} onValueChange={handleTabChange}>
      <div ref={sentinelRef} aria-hidden className="h-px" />
      <div
        className={cn(
          'sticky top-0 right-0 left-0 z-30 min-w-0 overflow-hidden rounded-lg border bg-card transition-shadow duration-200',
          isStuck && 'shadow-sm',
        )}>
        <TabsList aria-label="Resource sections" className="h-auto w-full gap-1 overflow-x-auto border-0">
          {tabs.map((tab) => (
            <TabsTrigger
              key={tab.label}
              value={tab.label}
              disabled={tab.disabled?.(resource) ?? false}
              className="h-[calc(var(--control-height)+0.5rem)] shrink-0 rounded-none px-4 after:inset-x-3 after:h-0.5 after:rounded-none data-[state=active]:bg-primary/10">
              {tab.Label ? <tab.Label /> : tab.label}
            </TabsTrigger>
          ))}
        </TabsList>
      </div>

      {tabs
        .filter((tab) => !(tab.disabled?.(resource) ?? false))
        .map((tab) => (
          <TabsContent key={tab.label} value={tab.label} className="mt-3">
            <tab.Content resource={resource} metadataChanged={metadataChanged} />
          </TabsContent>
        ))}
    </Tabs>
  );
};
