import { useState } from 'react';
import { Popover, PopoverContent, PopoverTrigger } from '../ui/popover';
import { Button } from '../ui/button';
import { Check, ChevronsUpDown, SearchX } from 'lucide-react';
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from '../ui/command';
import { cn, filterBySplit } from '@/lib/utils';
import { PluralResourceMap, ResourceType } from '@/api/types';
import { useRead } from '@/lib/hooks';
import { useResourceFilter } from '@/lib/atoms';

export const ResourceSelector = <T extends { id: string; name: string }>({
  type,
  selected,
  onSelect,
  disabled,
  align = 'start',
  placeholder,
  className,
}: {
  type: ResourceType;
  selected?: T | undefined;
  onSelect?: (item: T | undefined) => void;
  disabled?: boolean;
  align?: 'start' | 'center' | 'end';
  placeholder?: string;
  className?: string;
}) => {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');
  const resourceName = PluralResourceMap[type];
  const [filter, setFilter] = useResourceFilter<{ item: T }>(type);

  const items = Object.values(useRead(`list${resourceName}`).data?.data ?? {}).at(0) as T[];

  const selectedItem = filter?.item ?? selected;

  const name = items?.find((r) => r.id === selectedItem?.id)?.name;

  if (!items) return null;

  const filtered = filterBySplit(items, search, (item) => item.name).sort((a, b) => {
    if (a.name > b.name) {
      return 1;
    } else if (a.name < b.name) {
      return -1;
    } else {
      return 0;
    }
  });

  const isNoneSelected = !selectedItem;

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="ghost"
          className={cn(
            `flex justify-between gap-2 w-full max-w-[200px] bg-accent/60 hover:bg-accent/80 shadow-none`,
            className,
          )}
          disabled={disabled}>
          {name || (placeholder ?? `Select the resource`)}
          {!disabled && <ChevronsUpDown className="w-3 h-3" />}
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[300px] max-h-[300px] p-0 bg-background" align={align}>
        <Command shouldFilter={false}>
          <CommandInput
            placeholder={`Search ${PluralResourceMap[type]}`}
            className={cn(`h-9`, className)}
            value={search}
            onValueChange={setSearch}
          />
          <CommandList>
            <CommandEmpty className="flex justify-evenly items-center pt-3 pb-2">
              {`No ${PluralResourceMap[type]} Found`}
              <SearchX className="w-3 h-3" />
            </CommandEmpty>

            <CommandGroup>
              {!search && (
                <CommandItem
                  onSelect={() => {
                    onSelect && onSelect(undefined);
                    setFilter(null);
                    setOpen(false);
                  }}
                  role="option"
                  aria-selected={isNoneSelected}
                  className={cn(
                    'flex items-center justify-between cursor-pointer my-0.5 rounded-sm',
                    isNoneSelected && 'bg-accent/80',
                  )}>
                  <div className={cn('p-1', className)}>None</div>
                  <Check
                    className={cn('ml-2 h-4 w-4', isNoneSelected ? 'opacity-100' : 'opacity-0')}
                    aria-hidden="true"
                  />
                </CommandItem>
              )}
              {filtered.map((resource) => {
                const isSelected = selectedItem?.id === resource.id;

                return (
                  <CommandItem
                    key={resource.id}
                    onSelect={() => {
                      setFilter({ item: resource });
                      onSelect && onSelect(resource);
                      setOpen(false);
                    }}
                    role="option"
                    aria-selected={isSelected}
                    className={cn(
                      'flex items-center justify-between cursor-pointer my-0.5 rounded-sm',
                      isSelected && 'bg-accent/80',
                    )}>
                    <div className={cn('p-1', className)}>{resource.name}</div>
                    <Check
                      className={cn('ml-2 h-4 w-4', isSelected ? 'opacity-100' : 'opacity-0')}
                      aria-hidden="true"
                    />
                  </CommandItem>
                );
              })}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
};
