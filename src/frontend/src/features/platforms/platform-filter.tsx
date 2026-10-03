import { PlatformView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from '@/components/ui/command';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useRead } from '@/lib/hooks';
import { cn } from '@/lib/utils';
import { Check, Minus, Server } from 'lucide-react';
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
        <button
          type="button"
          className="inline-flex h-(--control-height) max-w-48 items-center gap-1.5 rounded-sm border border-border px-2 text-xs font-medium shadow-xs"
          title={`Clear ${selectedPlatform.name} platform filter`}
          onClick={() => setSelectedPlatformId(undefined)}>
          <Minus className="size-3 shrink-0" />
          <span className="truncate">{selectedPlatform.name}</span>
        </button>
      )}
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            type="button"
            variant="outline"
            className="h-(--control-height) shrink-0 rounded-sm px-2.5"
            disabled={isLoading}>
            <Server className="size-3.5" />
            Platform Filter
          </Button>
        </PopoverTrigger>
        <PopoverContent align="end" className="w-80 p-0 bg-background">
          <Command>
            <CommandInput placeholder="Search platforms..." />
            <CommandList className="max-h-72 overscroll-contain" onWheel={(event) => event.stopPropagation()}>
              <CommandEmpty>No platforms found.</CommandEmpty>
              <CommandGroup>
                {platforms.map((platform) => (
                  <CommandItem
                    key={platform.id}
                    value={`${platform.name} ${platform.id}`}
                    onSelect={() => selectPlatform(platform)}
                    className="cursor-pointer">
                    <StateIndicator value={platform.status} kind="platform" />
                    <span className="min-w-0 flex-1 truncate">{platform.name}</span>
                    <Check
                      className={cn('size-3.5', selectedPlatformId === platform.id ? 'opacity-100' : 'opacity-0')}
                    />
                  </CommandItem>
                ))}
              </CommandGroup>
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>
    </div>
  );
};
