import { useState } from 'react';
import { Popover, PopoverContent, PopoverTrigger } from '../ui/popover';
import { Button } from '../ui/button';
import { ChevronsUpDown, SearchX } from 'lucide-react';
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from '../ui/command';
import { filterBySplit } from '@/lib/utils';
import { PluralResourceMap, ResourceType } from '@/api/types';
import { useRead } from '@/lib/hooks';
import { useResourceFilter } from '@/lib/atoms';

export const ResourceSelector = <T extends { id: string; name: string }>({
  type,
  selected,
  onSelect,
  disabled,
  align,
  placeholder,
}: {
  type: ResourceType;
  selected?: T | undefined;
  onSelect?: (item: T | undefined) => void;
  disabled?: boolean;
  align?: 'start' | 'center' | 'end';
  placeholder?: string;
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

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="secondary"
          className="flex justify-start gap-2 w-fit max-w-[350px] bg-accent/60 hover:bg-accent/80"
          disabled={disabled}>
          {name || (placeholder ?? `Select the resource`)}
          {!disabled && <ChevronsUpDown className="w-3 h-3" />}
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[300px] max-h-[300px] p-0 bg-background" align={align}>
        <Command shouldFilter={false}>
          <CommandInput
            placeholder={`Search ${type}s`}
            className="h-9 border-transparent"
            value={search}
            onValueChange={setSearch}
          />
          <CommandList>
            <CommandEmpty className="flex justify-evenly items-center pt-3 pb-2">
              {`No ${type}s Found`}
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
                  className="flex items-center justify-between cursor-pointer">
                  <div className="p-1">None</div>
                </CommandItem>
              )}
              {filtered.map((resource) => (
                <CommandItem
                  key={resource.id}
                  onSelect={() => {
                    setFilter({ item: resource });

                    onSelect && onSelect(resource);
                    setOpen(false);
                  }}
                  className="flex items-center justify-between cursor-pointer">
                  <div className="p-1">{resource.name}</div>
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
};
