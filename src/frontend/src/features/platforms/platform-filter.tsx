import { PlatformView } from '@/api/generated/api.types';
import { ResourceFilterChip, ResourceFilterDropdown } from '@/components/custom/resource-filter';
import { CommandGroup, CommandItem } from '@/components/ui/command';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useRead } from '@/lib/hooks';
import { cn } from '@/lib/utils';
import { Check, Server } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useSearchParams } from 'react-router';

const PLATFORM_QUERY_KEY = 'platformId';

export const useResourcePlatformFilter = () => {
  const [searchParams, setSearchParams] = useSearchParams();
  const selectedPlatformId = searchParams.get(PLATFORM_QUERY_KEY) || undefined;

  const setSelectedPlatformId = (platformId?: string) => {
    setSearchParams((current) => {
      const next = new URLSearchParams(current);
      next.delete(PLATFORM_QUERY_KEY);
      if (platformId) {
        next.set(PLATFORM_QUERY_KEY, platformId);
      }
      return next;
    });
  };

  return { selectedPlatformId, setSelectedPlatformId };
};

export const ResourcePlatformFilter = () => {
  const [open, setOpen] = useState(false);
  const { selectedPlatformId, setSelectedPlatformId } = useResourcePlatformFilter();
  const { data, isLoading } = useRead('listPlatforms');
  const platforms = useMemo(() => data?.data.platforms ?? [], [data?.data.platforms]);
  const selectedPlatform = useMemo(
    () => platforms.find((platform) => platform.id === selectedPlatformId),
    [platforms, selectedPlatformId],
  );

  const selectPlatform = (platform: PlatformView) => {
    setSelectedPlatformId(platform.id);
    setOpen(false);
  };

  if (!isLoading && platforms.length === 0) return null;

  return (
    <div className="flex min-w-0 flex-wrap items-center gap-2">
      {selectedPlatform && (
        <ResourceFilterChip
          label={`Clear ${selectedPlatform.name} platform filter`}
          onClick={() => setSelectedPlatformId(undefined)}>
          {selectedPlatform.name}
        </ResourceFilterChip>
      )}
      <ResourceFilterDropdown
        label="Platform Filter"
        icon={Server}
        disabled={isLoading}
        open={open}
        onOpenChange={setOpen}
        contentClassName="w-80"
        searchPlaceholder="Search platforms..."
        emptyMessage="No platforms found.">
        <CommandGroup>
          {platforms.map((platform) => (
            <CommandItem
              key={platform.id}
              value={`${platform.name} ${platform.id}`}
              onSelect={() => selectPlatform(platform)}
              className="cursor-pointer">
              <StateIndicator value={platform.status} kind="platform" />
              <span className="min-w-0 flex-1 truncate">{platform.name}</span>
              <Check className={cn('size-3.5', selectedPlatformId === platform.id ? 'opacity-100' : 'opacity-0')} />
            </CommandItem>
          ))}
        </CommandGroup>
      </ResourceFilterDropdown>
    </div>
  );
};
