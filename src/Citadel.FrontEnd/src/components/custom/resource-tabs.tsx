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
  const enabledTabs = useMemo(
    () => tabs.filter((tab) => !(tab.disabled?.(resource) ?? false)),
    [tabs, resource],
  );
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
    <Tabs value={effectiveActiveTab} onValueChange={handleTabChange} className="gap-4">
      <div ref={sentinelRef} aria-hidden className="h-px" />
      <div
        className={cn(
          'sticky top-0 z-30 bg-background left-0 right-0 transition-all duration-200',
          isStuck ? '-mx-4' : 'mx-0',
        )}>
        <TabsList className={cn('w-full overflow-x-auto', isStuck && 'border-b rounded-none py-2')}>
          {tabs.map((tab) => (
            <TabsTrigger
              key={tab.label}
              value={tab.label}
              className="text-xs"
              disabled={tab.disabled?.(resource) ?? false}>
              {tab.label}
            </TabsTrigger>
          ))}
        </TabsList>
      </div>

      {tabs
        .filter((tab) => !(tab.disabled?.(resource) ?? false))
        .map((tab) => (
          <TabsContent key={tab.label} value={tab.label}>
            <tab.Content resource={resource} metadataChanged={metadataChanged} />
          </TabsContent>
        ))}
    </Tabs>
  );
};
