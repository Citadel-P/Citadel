import { GlobalSearchCategory, type GlobalSearchItem, SearchStatusTone } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { CommandDialog, CommandGroup, CommandInput, CommandItem, CommandList } from '@/components/ui/command';
import { useDebounce } from '@/hooks/useDebounce';
import { cn } from '@/lib/utils';
import { useRead } from '@/lib/hooks';
import { CornerUpLeft, LoaderCircle, Search } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router';
import { globalSearchCategories, globalSearchResourceMetadata } from './global-search-metadata';

const MINIMUM_QUERY_LENGTH = 2;
const SEARCH_DELAY_MS = 275;

const statusToneClass: Record<SearchStatusTone, string> = {
  [SearchStatusTone.Positive]: 'bg-emerald-500',
  [SearchStatusTone.Negative]: 'bg-red-500',
  [SearchStatusTone.Warning]: 'bg-amber-500',
  [SearchStatusTone.Info]: 'bg-blue-500',
  [SearchStatusTone.Neutral]: 'bg-gray-400',
};

export function GlobalSearch() {
  const navigate = useNavigate();
  const triggerRef = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const normalizedQuery = query.trim();
  const debouncedQuery = useDebounce(normalizedQuery, SEARCH_DELAY_MS);
  const hasSearchQuery = normalizedQuery.length >= MINIMUM_QUERY_LENGTH;
  const queryIsSettled = debouncedQuery === normalizedQuery;
  const isMac = useMemo(() => typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform), []);

  const search = useRead(
    'globalSearch',
    { query: { q: debouncedQuery, limitPerType: 5 } },
    {
      enabled: open && queryIsSettled && debouncedQuery.length >= MINIMUM_QUERY_LENGTH,
      placeholderData: (previous) => previous,
    },
  );

  const response = search.data?.data;
  const groups = queryIsSettled && response?.query === debouncedQuery ? response.groups : undefined;

  const setDialogOpen = useCallback((nextOpen: boolean) => {
    setOpen(nextOpen);
    if (!nextOpen) {
      setQuery('');
      window.setTimeout(() => triggerRef.current?.focus(), 0);
    }
  }, []);

  useEffect(() => {
    const handleShortcut = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.key.toLowerCase() !== 'k' || (!event.ctrlKey && !event.metaKey)) {
        return;
      }

      const target = event.target instanceof Element ? event.target : null;
      if (
        target?.closest('input, textarea, select, [contenteditable="true"], .monaco-editor, [role="dialog"]') ||
        document.querySelector('[role="dialog"][data-state="open"]')
      ) {
        return;
      }

      event.preventDefault();
      setOpen(true);
    };

    document.addEventListener('keydown', handleShortcut);
    return () => document.removeEventListener('keydown', handleShortcut);
  }, []);

  const navigateAndClose = useCallback(
    (path: string) => {
      setDialogOpen(false);
      navigate(path);
    },
    [navigate, setDialogOpen],
  );

  return (
    <>
      <Button
        ref={triggerRef}
        type="button"
        variant="outline"
        size="sm"
        onClick={() => setOpen(true)}
        aria-label="Search resources"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-keyshortcuts="Control+K Meta+K"
        data-state={open ? 'open' : 'closed'}
        className="group h-9 w-9 justify-center overflow-hidden border-border/70 bg-muted/30 px-0 font-normal text-muted-foreground shadow-xs hover:border-border hover:bg-accent/60 hover:text-foreground data-[state=open]:border-ring/50 data-[state=open]:bg-accent data-[state=open]:text-foreground sm:w-56 sm:justify-start sm:px-3 lg:w-64">
        <Search className="size-4 shrink-0 transition-colors group-hover:text-foreground" />
        <span className="hidden min-w-0 flex-1 truncate text-left sm:inline">Search resources...</span>
        <kbd className="pointer-events-none ml-auto hidden h-5 shrink-0 items-center gap-1 rounded border border-border/80 bg-background px-1.5 font-mono text-[10px] font-medium text-muted-foreground shadow-xs lg:inline-flex">
          <span>{isMac ? '⌘' : 'Ctrl'}</span>
          <span>K</span>
        </kbd>
      </Button>

      <CommandDialog
        open={open}
        onOpenChange={setDialogOpen}
        title="Search resources"
        description="Find resources you can access"
        shouldFilter={false}
        overlayClassName="bg-black/35 backdrop-blur-[2px] dark:bg-black/55"
        className="top-[42%] border-border/80 bg-background shadow-2xl supports-[backdrop-filter]:bg-background sm:max-w-xl">
        <CommandInput value={query} onValueChange={setQuery} placeholder="Search resources..." />
        <CommandList className="max-h-[min(65vh,32rem)]">
          {!hasSearchQuery ? (
            <CommandGroup heading="Resources">
              {globalSearchCategories.map((category) => {
                const Icon = category.icon;
                return (
                  <CommandItem
                    key={category.category}
                    value={category.category}
                    onSelect={() => navigateAndClose(category.listPath)}>
                    <Icon className="size-4" />
                    <span>{category.label}</span>
                  </CommandItem>
                );
              })}
            </CommandGroup>
          ) : null}

          {hasSearchQuery && (!queryIsSettled || (search.isFetching && !groups)) ? (
            <div className="flex h-24 items-center justify-center gap-2 text-sm text-muted-foreground">
              <LoaderCircle className="size-4 animate-spin" />
              Searching
            </div>
          ) : null}

          {hasSearchQuery && queryIsSettled && search.isError ? (
            <div className="flex h-28 flex-col items-center justify-center gap-3 px-4 text-center">
              <p className="text-sm text-muted-foreground">Search could not be completed.</p>
              <Button type="button" variant="outline" size="sm" onClick={() => search.refetch()}>
                Retry
              </Button>
            </div>
          ) : null}

          {hasSearchQuery && queryIsSettled && !search.isError && groups?.length === 0 && !search.isFetching ? (
            <div className="px-4 py-8 text-center text-sm text-muted-foreground">
              No resources found for &quot;{normalizedQuery}&quot;
            </div>
          ) : null}

          {hasSearchQuery && queryIsSettled && !search.isError
            ? groups?.map((group) => (
                <CommandGroup key={group.category} heading={group.category}>
                  {group.items.map((item) => (
                    <SearchResult
                      key={`${item.resourceType}:${item.id}`}
                      item={item}
                      showType={
                        group.category === GlobalSearchCategory.Backups ||
                        group.category === GlobalSearchCategory.Builds
                      }
                      onNavigate={navigateAndClose}
                    />
                  ))}
                </CommandGroup>
              ))
            : null}

          {hasSearchQuery && queryIsSettled && search.isFetching && groups ? (
            <div className="sticky bottom-0 flex h-7 items-center justify-center gap-2 border-t bg-popover/95 text-xs text-muted-foreground">
              <LoaderCircle className="size-3 animate-spin" />
              Updating
            </div>
          ) : null}
        </CommandList>
      </CommandDialog>
    </>
  );
}

function SearchResult({
  item,
  showType,
  onNavigate,
}: {
  item: GlobalSearchItem;
  showType: boolean;
  onNavigate: (path: string) => void;
}) {
  const metadata = globalSearchResourceMetadata[item.resourceType];
  const Icon = metadata.icon;

  return (
    <CommandItem
      value={`${item.resourceType}:${item.id}:${item.name}:${item.secondaryText ?? ''}`}
      onSelect={() => onNavigate(metadata.getDetailsPath(item.id))}
      className="min-h-12 gap-3">
      <Icon className="size-4 self-start mt-0.5" />
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate font-medium">{item.name}</span>
          {showType ? (
            <span className="shrink-0 text-[11px] font-normal text-muted-foreground">{metadata.singularLabel}</span>
          ) : null}
        </div>
        {item.secondaryText ? (
          <div className="mt-0.5 truncate text-xs text-muted-foreground">{item.secondaryText}</div>
        ) : null}
      </div>
      {item.status ? (
        <div className="flex shrink-0 items-center gap-1.5 text-xs text-muted-foreground">
          <span className={cn('size-2 rounded-full', statusToneClass[item.status.tone])} />
          <span>{item.status.label}</span>
        </div>
      ) : null}
      {item.parent ? (
        <button
          type="button"
          className="flex max-w-32 shrink-0 items-center gap-1 rounded-sm px-1.5 py-1 text-xs text-muted-foreground hover:bg-background hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          aria-label={`Open parent ${item.parent.name}`}
          onPointerDown={(event) => {
            event.preventDefault();
            event.stopPropagation();
          }}
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            const parentMetadata = globalSearchResourceMetadata[item.parent!.resourceType];
            onNavigate(parentMetadata.getDetailsPath(item.parent!.id));
          }}>
          <CornerUpLeft className="size-3" />
          <span className="truncate">{item.parent.name}</span>
        </button>
      ) : null}
    </CommandItem>
  );
}
