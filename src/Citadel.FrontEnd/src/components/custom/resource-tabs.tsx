import { useLocalStorage, useStickySentinel } from '@/lib/hooks';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/tabs';
import { cn } from '@/lib/utils';
import { RequiredFormFields, TabElement } from '@/pages/types';
import { useSegmentTitle } from '@/lib/atoms';
import { useEffect } from 'react';

export const ResourceTabs = ({
  localKey,
  resource,
  tabs,
  metadataChanged,
}: {
  localKey: string;
  resource: RequiredFormFields;
  tabs: TabElement<any>[];
  metadataChanged?: boolean;
}) => {
  const [_, setSegmentTitle] = useSegmentTitle();

  useEffect(() => {
    if (resource?.name) {
      setSegmentTitle({ action: tabs[0]?.label, name: resource?.name });
    }
  }, [resource?.name, setSegmentTitle]);

  const [activeTab, setActiveTab] = useLocalStorage(localKey, tabs[0]?.label ?? 'default');
  const { sentinelRef, isStuck } = useStickySentinel();

  return (
    <Tabs value={activeTab} onValueChange={setActiveTab} className="gap-4">
      <div ref={sentinelRef} aria-hidden className="h-px" />
      <div
        className={cn(
          'sticky top-11.5 z-30 bg-background left-0 right-0 transition-all duration-200',
          isStuck ? '-mx-4' : 'mx-0',
        )}>
        <TabsList className={cn('w-full overflow-x-auto', isStuck && 'border-b rounded-none py-2')}>
          {tabs.map((tab) => (
            <TabsTrigger key={tab.label} value={tab.label} className="text-xs">
              {tab.label}
            </TabsTrigger>
          ))}
        </TabsList>
      </div>

      {tabs.map((tab) => (
        <TabsContent key={tab.label} value={tab.label}>
          <tab.Content resource={resource} metadataChanged={metadataChanged} />
        </TabsContent>
      ))}
    </Tabs>
  );
};
