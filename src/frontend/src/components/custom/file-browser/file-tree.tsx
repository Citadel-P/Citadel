import { AlertMessage } from '@/components/custom/alert-message';
import { Button } from '@/components/ui/button';
import { byteTransform } from '@/lib/bytes.helper';
import { cn } from '@/lib/utils';
import { useQuery } from '@tanstack/react-query';
import { ChevronDown, ChevronRight, File, Folder, FolderOpen, GitCommit, Link2, Loader2 } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { BrowserEntry, LoadBrowserDirectory } from './file-browser.types';

type FileTreeProps = {
  queryKey: readonly unknown[];
  rootPath: string;
  rootName: string;
  loadDirectory: LoadBrowserDirectory;
  enabled?: boolean;
  refreshVersion?: number;
  selectedPath?: string | null;
  initialPath?: string | null;
  highlightedPaths?: readonly string[];
  onSelect?: (entry: BrowserEntry) => void;
  renderEntryActions?: (entry: BrowserEntry) => React.ReactNode;
  getErrorTitle?: (error: unknown) => string;
};

export function FileTree({
  queryKey,
  rootPath,
  rootName,
  loadDirectory,
  enabled = true,
  refreshVersion = 0,
  selectedPath,
  initialPath,
  highlightedPaths = [],
  onSelect,
  renderEntryActions,
  getErrorTitle,
}: FileTreeProps) {
  const highlighted = useMemo(() => new Set(highlightedPaths), [highlightedPaths]);

  return (
    <div className="min-w-0 py-2">
      <FolderNode
        queryKey={queryKey}
        path={rootPath}
        name={rootName}
        depth={0}
        enabled={enabled}
        refreshVersion={refreshVersion}
        selectedPath={selectedPath}
        initialPath={initialPath}
        highlightedPaths={highlighted}
        loadDirectory={loadDirectory}
        onSelect={onSelect}
        renderEntryActions={renderEntryActions}
        getErrorTitle={getErrorTitle}
        initiallyExpanded
      />
    </div>
  );
}

type FolderNodeProps = {
  queryKey: readonly unknown[];
  path: string;
  name: string;
  depth: number;
  enabled: boolean;
  refreshVersion: number;
  selectedPath?: string | null;
  initialPath?: string | null;
  highlightedPaths: ReadonlySet<string>;
  loadDirectory: LoadBrowserDirectory;
  onSelect?: (entry: BrowserEntry) => void;
  renderEntryActions?: (entry: BrowserEntry) => React.ReactNode;
  getErrorTitle?: (error: unknown) => string;
  initiallyExpanded?: boolean;
};

function FolderNode({
  queryKey,
  path,
  name,
  depth,
  enabled,
  refreshVersion,
  selectedPath,
  initialPath,
  highlightedPaths,
  loadDirectory,
  onSelect,
  renderEntryActions,
  getErrorTitle,
  initiallyExpanded = false,
}: FolderNodeProps) {
  const shouldRevealInitialPath = isAncestor(path, initialPath);
  const [expanded, setExpanded] = useState(initiallyExpanded || shouldRevealInitialPath);
  const query = useQuery({
    queryKey: [...queryKey, refreshVersion, path],
    queryFn: ({ signal }) => loadDirectory(path, signal),
    enabled: enabled && expanded,
    staleTime: 30_000,
    meta: { suppressErrorToast: true },
  });
  const entries = useMemo(() => sortEntries(query.data?.entries ?? []), [query.data?.entries]);
  const isLoading = query.isLoading || (query.isFetching && !query.data);

  useEffect(() => {
    if (shouldRevealInitialPath) setExpanded(true);
  }, [shouldRevealInitialPath]);

  useEffect(() => {
    if (!initialPath || !onSelect || selectedPath) return;

    const entry = entries.find((candidate) => candidate.path === initialPath && candidate.type !== 'directory');
    if (entry) onSelect(entry);
  }, [entries, initialPath, onSelect, selectedPath]);

  const folderEntry: BrowserEntry = { name, path, type: 'directory' };
  const isRoot = depth === 0;

  return (
    <div>
      <div
        className={cn(
          'group flex min-h-8 items-center gap-1 rounded-sm px-1 text-sm hover:bg-muted/60',
          selectedPath === path && 'bg-muted',
        )}
        style={{ paddingLeft: depth * 14 }}>
        <Button
          type="button"
          variant="ghost"
          size="icon-xs"
          className="size-6 shrink-0"
          aria-label={`${expanded ? 'Collapse' : 'Expand'} ${name}`}
          disabled={isLoading}
          onClick={() => setExpanded((value) => !value)}>
          {isLoading ? (
            <Loader2 className="size-3 animate-spin" />
          ) : expanded ? (
            <ChevronDown className="size-3.5" />
          ) : (
            <ChevronRight className="size-3.5" />
          )}
        </Button>
        {expanded ? (
          <FolderOpen className="size-4 shrink-0 text-blue-500" />
        ) : (
          <Folder className="size-4 shrink-0 text-blue-500" />
        )}
        <button
          type="button"
          className={cn('min-w-0 flex-1 truncate text-left', isRoot && 'font-medium')}
          title={path || name}
          onClick={() => onSelect?.(folderEntry)}>
          {name}
        </button>
        {!isRoot && renderEntryActions?.(folderEntry)}
      </div>

      {expanded && (
        <div>
          {query.error ? (
            <TreeProblem error={query.error} depth={depth + 1} getErrorTitle={getErrorTitle} />
          ) : isLoading ? (
            <TreeStatus depth={depth + 1} icon={<Loader2 className="size-3 animate-spin" />} text="Loading" />
          ) : entries.length === 0 ? (
            <TreeStatus depth={depth + 1} text="Empty" />
          ) : (
            entries.map((entry) =>
              entry.type === 'directory' ? (
                <FolderNode
                  key={entry.path}
                  queryKey={queryKey}
                  path={entry.path}
                  name={entry.name}
                  depth={depth + 1}
                  enabled={enabled}
                  refreshVersion={refreshVersion}
                  selectedPath={selectedPath}
                  initialPath={initialPath}
                  highlightedPaths={highlightedPaths}
                  loadDirectory={loadDirectory}
                  onSelect={onSelect}
                  renderEntryActions={renderEntryActions}
                  getErrorTitle={getErrorTitle}
                />
              ) : (
                <FileNode
                  key={entry.path}
                  entry={entry}
                  depth={depth + 1}
                  selected={selectedPath === entry.path}
                  highlighted={highlightedPaths.has(entry.path)}
                  onSelect={onSelect}
                  actions={renderEntryActions?.(entry)}
                />
              ),
            )
          )}

          {query.data?.isTruncated && <TreeStatus depth={depth + 1} text="Directory listing was truncated." />}
        </div>
      )}
    </div>
  );
}

function FileNode({
  entry,
  depth,
  selected,
  highlighted,
  onSelect,
  actions,
}: {
  entry: BrowserEntry;
  depth: number;
  selected: boolean;
  highlighted: boolean;
  onSelect?: (entry: BrowserEntry) => void;
  actions?: React.ReactNode;
}) {
  return (
    <div
      className={cn(
        'group flex min-h-8 items-center gap-2 rounded-sm px-1 text-sm hover:bg-muted/60',
        selected && 'bg-muted',
      )}
      style={{ paddingLeft: depth * 14 + 30 }}>
      {renderEntryIcon(entry.type)}
      <button
        type="button"
        className="min-w-0 flex-1 truncate text-left"
        title={entry.secondaryText ? `${entry.name} -> ${entry.secondaryText}` : entry.path}
        onClick={() => onSelect?.(entry)}>
        {entry.name}
      </button>
      {highlighted && (
        <span className="hidden shrink-0 rounded-sm bg-secondary px-1.5 py-0.5 text-[10px] text-secondary-foreground sm:inline">
          Stack source
        </span>
      )}
      {entry.secondaryText && (
        <span className="hidden max-w-36 truncate text-xs text-muted-foreground lg:inline">
          {'->'} {entry.secondaryText}
        </span>
      )}
      <span className="hidden shrink-0 text-xs text-muted-foreground sm:inline">{formatSize(entry)}</span>
      {entry.metadataText && (
        <span className="hidden shrink-0 text-xs text-muted-foreground lg:inline">{entry.metadataText}</span>
      )}
      {actions}
    </div>
  );
}

function TreeProblem({
  error,
  depth,
  getErrorTitle,
}: {
  error: unknown;
  depth: number;
  getErrorTitle?: (error: unknown) => string;
}) {
  const problem = (error as any)?.error;
  const status = problem?.status;
  return (
    <div className="py-1 pr-2" style={{ paddingLeft: depth * 14 + 30 }}>
      <AlertMessage
        type={status === 403 ? 'warning' : 'error'}
        title={getErrorTitle?.(error) ?? (status === 403 ? 'Permission denied' : 'Browse failed')}>
        {problem?.detail ??
          problem?.title ??
          (error instanceof Error ? error.message : 'Citadel could not list this path.')}
      </AlertMessage>
    </div>
  );
}

function TreeStatus({ depth, text, icon }: { depth: number; text: string; icon?: React.ReactNode }) {
  return (
    <div
      className="flex min-h-8 items-center gap-2 rounded-sm px-1 text-xs text-muted-foreground"
      style={{ paddingLeft: depth * 14 + 30 }}>
      {icon}
      <span>{text}</span>
    </div>
  );
}

function sortEntries(entries: BrowserEntry[]) {
  return [...entries].sort((left, right) => {
    const bucket = entryBucket(left.type) - entryBucket(right.type);
    if (bucket !== 0) return bucket;
    const insensitive = left.name.localeCompare(right.name, undefined, { sensitivity: 'base' });
    return insensitive || left.name.localeCompare(right.name);
  });
}

function entryBucket(type: BrowserEntry['type']) {
  switch (type) {
    case 'directory':
      return 0;
    case 'file':
      return 1;
    case 'symlink':
      return 2;
    case 'submodule':
      return 3;
  }
}

function renderEntryIcon(type: BrowserEntry['type']) {
  switch (type) {
    case 'symlink':
      return <Link2 className="size-4 shrink-0 text-muted-foreground" />;
    case 'submodule':
      return <GitCommit className="size-4 shrink-0 text-amber-500" />;
    default:
      return <File className="size-4 shrink-0 text-muted-foreground" />;
  }
}

function formatSize(entry: BrowserEntry) {
  if (entry.type === 'directory' || entry.type === 'submodule') return '';
  const size = typeof entry.size === 'string' ? Number(entry.size) : entry.size;
  return size == null || Number.isNaN(size) ? '-' : byteTransform(size, 2);
}

function isAncestor(folderPath: string, targetPath?: string | null) {
  if (!targetPath) return false;
  if (folderPath === '' || folderPath === '/') return true;
  const separator = folderPath.endsWith('/') ? '' : '/';
  return targetPath === folderPath || targetPath.startsWith(`${folderPath}${separator}`);
}
