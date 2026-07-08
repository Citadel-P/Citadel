import { TagSummaryView, TagView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from '@/components/ui/command';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { useMutate, useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { LoaderCircle, Minus, Plus, Tag } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useSearchParams } from 'react-router';
import { getTagTextColor } from './tag-colors';
import { toast } from 'sonner';

const TAG_QUERY_KEY = 'tags';
const normalizeTagName = (name: string) => name.trim().toLowerCase();

export const useResourceTagFilter = () => {
  const [searchParams, setSearchParams] = useSearchParams();

  const selectedTagNames = useMemo(
    () => searchParams.getAll(TAG_QUERY_KEY).filter((value) => value.trim().length > 0),
    [searchParams],
  );

  const setSelectedTagNames = (tagNames: string[]) => {
    setSearchParams((current) => {
      const next = new URLSearchParams(current);
      next.delete(TAG_QUERY_KEY);
      tagNames.forEach((tagName) => next.append(TAG_QUERY_KEY, tagName));
      return next;
    });
  };

  return { selectedTagNames, setSelectedTagNames };
};

export const ResourceTagFilter = () => {
  const [open, setOpen] = useState(false);
  const { selectedTagNames, setSelectedTagNames } = useResourceTagFilter();
  const { data, isLoading } = useRead('listTags');
  const tags = useMemo(() => data?.data.tags ?? [], [data?.data.tags]);
  const selectedTags = useMemo(() => {
    const selected = new Set(selectedTagNames.map(normalizeTagName));
    return tags.filter((tag) => selected.has(normalizeTagName(tag.name)));
  }, [selectedTagNames, tags]);

  const availableTags = useMemo(() => {
    const selected = new Set(selectedTagNames.map(normalizeTagName));
    return tags.filter((tag) => !selected.has(normalizeTagName(tag.name)));
  }, [selectedTagNames, tags]);

  const selectTag = (tagName: string) => setSelectedTagNames([...selectedTagNames, tagName]);
  const removeTag = (tagName: string) =>
    setSelectedTagNames(selectedTagNames.filter((name) => normalizeTagName(name) !== normalizeTagName(tagName)));

  if (!isLoading && tags.length === 0) return null;

  return (
    <div className="flex min-w-0 flex-wrap items-center gap-2">
      {selectedTags.map((tag) => (
        <button
          key={tag.id}
          type="button"
          className="inline-flex h-9 max-w-40 items-center gap-1.5 rounded-sm border border-border px-2 text-xs font-medium shadow-xs"
          style={{ backgroundColor: tag.color, color: getTagTextColor(tag.color) }}
          title={`Remove ${tag.name} tag filter`}
          onClick={() => removeTag(tag.name)}>
          <Minus className="size-3 shrink-0" />
          <span className="truncate">{tag.name}</span>
        </button>
      ))}
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button type="button" variant="outline" className="h-9 shrink-0 rounded-sm px-2.5" disabled={isLoading}>
            <Tag className="size-3.5" />
            Tag Filter
          </Button>
        </PopoverTrigger>
        <PopoverContent align="end" className="w-72 p-0 bg-background">
          <Command filter={tagSearchFilter}>
            <CommandInput placeholder="Search tags..." />
            <CommandList className="max-h-72 overscroll-contain" onWheel={(event) => event.stopPropagation()}>
              <CommandEmpty>{availableTags.length === 0 ? 'No more tags.' : 'No tags found.'}</CommandEmpty>
              <CommandGroup>
                {availableTags.map((tag) => (
                  <CommandItem
                    key={tag.id}
                    value={tag.name}
                    onSelect={() => selectTag(tag.name)}
                    className="cursor-pointer">
                    <span
                      className="size-3 shrink-0 rounded-full border border-border"
                      style={{ backgroundColor: tag.color }}
                    />
                    <span className="truncate">{tag.name}</span>
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

export const ResourceTagSelector = ({
  value,
  onChange,
  disabled,
}: {
  value?: string[] | null;
  onChange: (tagIds: string[]) => void;
  disabled?: boolean;
}) => {
  const [open, setOpen] = useState(false);
  const { data, isLoading } = useRead('listTags');
  const tags = useMemo(() => data?.data.tags ?? [], [data?.data.tags]);
  const selectedTagIds = useMemo(() => value ?? [], [value]);
  const selectedTags = useMemo(() => {
    const selected = new Set(selectedTagIds);
    return tags.filter((tag) => selected.has(tag.id));
  }, [selectedTagIds, tags]);

  const availableTags = useMemo(() => {
    const selected = new Set(selectedTagIds);
    return tags.filter((tag) => !selected.has(tag.id));
  }, [selectedTagIds, tags]);

  const selectTag = (tagId: string) => onChange([...selectedTagIds, tagId]);
  const removeTag = (tagId: string) => onChange(selectedTagIds.filter((id) => id !== tagId));
  const isDisabled = disabled || isLoading || tags.length === 0;

  return (
    <div className="flex min-w-0 max-w-100 flex-wrap items-center gap-2">
      {selectedTags.map((tag) => (
        <button
          key={tag.id}
          type="button"
          className="inline-flex h-8 max-w-36 items-center gap-1.5 rounded-sm border border-border px-2 text-xs font-medium shadow-xs"
          style={{ backgroundColor: tag.color, color: getTagTextColor(tag.color) }}
          title={`Remove ${tag.name} tag`}
          disabled={disabled}
          onClick={() => removeTag(tag.id)}>
          <Minus className="size-3 shrink-0" />
          <span className="truncate">{tag.name}</span>
        </button>
      ))}
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button type="button" size="icon-sm" variant="outline" className="rounded-sm" disabled={isDisabled}>
            {isLoading ? <LoaderCircle className="size-3.5 animate-spin" /> : <Plus className="size-3.5" />}
            <span className="sr-only">Add tag</span>
          </Button>
        </PopoverTrigger>
        <PopoverContent align="start" className="w-72 p-0 bg-background">
          <Command filter={tagSearchFilter}>
            <CommandInput placeholder="Search tags..." />
            <CommandList className="max-h-72 overscroll-contain" onWheel={(event) => event.stopPropagation()}>
              <CommandEmpty>{availableTags.length === 0 ? 'No more tags.' : 'No tags found.'}</CommandEmpty>
              <CommandGroup>
                {availableTags.map((tag) => (
                  <CommandItem
                    key={tag.id}
                    value={tag.name}
                    onSelect={() => selectTag(tag.id)}
                    className="cursor-pointer">
                    <span
                      className="size-3 shrink-0 rounded-full border border-border"
                      style={{ backgroundColor: tag.color }}
                    />
                    <span className="truncate">{tag.name}</span>
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

export const TagChips = ({ tags, max = 3 }: { tags?: TagSummaryView[] | TagView[] | null; max?: number }) => {
  const visibleTags = tags?.slice(0, max) ?? [];
  const hiddenCount = Math.max((tags?.length ?? 0) - visibleTags.length, 0);

  if (!tags || tags.length === 0) {
    return <span className="text-muted-foreground text-sm">-</span>;
  }

  return (
    <div className="flex max-w-64 flex-wrap items-center gap-1.5">
      {visibleTags.map((tag) => (
        <span
          key={tag.id}
          className="inline-flex max-w-32 items-center gap-1 rounded-sm bg-muted/40 px-2 py-1 text-[13px] text-foreground"
          style={{ backgroundColor: tag.color, color: getTagTextColor(tag.color) }}
          title={tag.name}>
          <span className="truncate">{tag.name}</span>
        </span>
      ))}
      {hiddenCount > 0 && <span className="text-muted-foreground text-xs">+{hiddenCount}</span>}
    </div>
  );
};

type EditableResourceType = 'Deployment' | 'Stack' | 'GitRepository' | 'Platform' | 'Registry' | 'AutomationAction';

const replaceTagEndpoint = {
  Deployment: 'replaceDeploymentTags',
  Stack: 'replaceStackTags',
  GitRepository: 'replaceGitRepositoryTags',
  Platform: 'replacePlatformTags',
  Registry: 'replaceRegistryTags',
  AutomationAction: 'replaceAutomationActionTags',
} as const;

export const ResourceHeaderTagsEditor = ({
  resourceType,
  resourceId,
  tags,
  disabled,
}: {
  resourceType: EditableResourceType;
  resourceId: string;
  tags?: TagSummaryView[] | null;
  disabled?: boolean;
}) => {
  const [open, setOpen] = useState(false);
  const queryClient = useQueryClient();
  const { data, isLoading } = useRead('listTags');
  const replaceTags = useMutate(replaceTagEndpoint[resourceType] as any);
  const allTags = useMemo(() => data?.data.tags ?? [], [data?.data.tags]);
  const initialTagIds = useMemo(() => tags?.map((tag) => tag.id) ?? [], [tags]);
  const initialTagIdsKey = useMemo(() => getTagIdsKey(initialTagIds), [initialTagIds]);
  const [draft, setDraft] = useState(() => ({ key: initialTagIdsKey, tagIds: initialTagIds }));
  const selectedTagIds = draft.key === initialTagIdsKey ? draft.tagIds : initialTagIds;

  const selectedTags = useMemo(() => {
    const knownTags = new Map(allTags.map((tag) => [tag.id, tag]));
    return selectedTagIds
      .map((tagId) => knownTags.get(tagId) ?? tags?.find((tag) => tag.id === tagId))
      .filter((tag): tag is TagSummaryView | TagView => Boolean(tag));
  }, [allTags, selectedTagIds, tags]);

  const availableTags = useMemo(() => {
    const selected = new Set(selectedTagIds);
    return allTags.filter((tag) => !selected.has(tag.id));
  }, [allTags, selectedTagIds]);

  const replace = async (tagIds: string[], successMessage: string) => {
    setDraft({ key: initialTagIdsKey, tagIds });
    await replaceTags.mutateAsync(buildReplaceVariables(resourceType, resourceId, tagIds) as any);
    toast.success(successMessage);
    await queryClient.invalidateQueries();
  };

  const selectTag = (tagId: string) => replace([...selectedTagIds, tagId], 'Tag added');
  const removeTag = (tagId: string) =>
    replace(
      selectedTagIds.filter((id) => id !== tagId),
      'Tag removed',
    );
  const isDisabled = disabled || isLoading || replaceTags.isPending;

  if (!isLoading && allTags.length === 0 && selectedTags.length === 0) return null;

  return (
    <div className="flex min-w-0 flex-wrap items-center gap-2">
      <span className="text-xs font-medium text-muted-foreground">Tags:</span>
      {selectedTags.map((tag) => (
        <button
          key={tag.id}
          type="button"
          className="inline-flex h-8 max-w-36 items-center gap-1.5 rounded-sm border border-border px-2 text-xs font-medium shadow-xs"
          style={{ backgroundColor: tag.color, color: getTagTextColor(tag.color) }}
          title={`Remove ${tag.name} tag`}
          disabled={isDisabled}
          onClick={() => removeTag(tag.id)}>
          <Minus className="size-3 shrink-0" />
          <span className="truncate">{tag.name}</span>
        </button>
      ))}
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button type="button" size="icon-sm" variant="outline" className="rounded-sm" disabled={isDisabled}>
            {replaceTags.isPending ? <LoaderCircle className="size-3.5 animate-spin" /> : <Plus className="size-3.5" />}
            <span className="sr-only">Add tag</span>
          </Button>
        </PopoverTrigger>
        <PopoverContent align="end" className="w-72 p-0 bg-background">
          <Command filter={tagSearchFilter}>
            <CommandInput placeholder="Search tags..." />
            <CommandList className="max-h-72 overscroll-contain" onWheel={(event) => event.stopPropagation()}>
              <CommandEmpty>{availableTags.length === 0 ? 'No more tags.' : 'No tags found.'}</CommandEmpty>
              <CommandGroup>
                {availableTags.map((tag) => (
                  <CommandItem
                    key={tag.id}
                    value={tag.name}
                    onSelect={() => selectTag(tag.id)}
                    className="cursor-pointer">
                    <span
                      className="size-3 shrink-0 rounded-full border border-border"
                      style={{ backgroundColor: tag.color }}
                    />
                    <span className="truncate">{tag.name}</span>
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

const getTagIdsKey = (tagIds: string[]) => [...tagIds].sort().join('|');

const tagSearchFilter = (value: string, search: string) => {
  if (!search.trim()) return 1;
  return value.toLowerCase().includes(search.toLowerCase()) ? 1 : 0;
};

const buildReplaceVariables = (resourceType: EditableResourceType, resourceId: string, tagIds: string[]) => {
  const data = { tagIds };

  switch (resourceType) {
    case 'Deployment':
      return { deploymentId: resourceId, data };
    case 'Stack':
      return { stackId: resourceId, data };
    case 'GitRepository':
    case 'Platform':
    case 'Registry':
    case 'AutomationAction':
      return { id: resourceId, data };
  }
};
