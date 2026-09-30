import {
  GitEntryTypeView,
  GitFileContent,
  GitRepositoryRefView,
  AuthorizedGitRepositoryView,
} from '@/api/generated/api.types';
import { useApiClientContext } from '@/api/api-client-context';
import { AlertMessage } from '@/components/custom/alert-message';
import { BrowserEntry, FileBrowser } from '@/components/custom/file-browser';
import { Button } from '@/components/ui/button';
import { useRead } from '@/lib/hooks';
import { MonacoEditor } from '@/lib/monaco';
import { cn } from '@/lib/utils';
import { ExternalLink, GitBranch, GitCommitHorizontal, Loader2 } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import { getRepositoryFileLanguage } from './file-language';

type RepositoryBrowserProps = {
  repository: AuthorizedGitRepositoryView;
  branch?: string | null;
  commitSha?: string | null;
  initialPath?: string | null;
  highlightedPaths?: readonly string[];
  showToolbar?: boolean;
  className?: string;
};

export function RepositoryBrowser({
  repository,
  branch,
  commitSha,
  initialPath,
  highlightedPaths,
  showToolbar = true,
  className,
}: RepositoryBrowserProps) {
  const { apiClient } = useApiClientContext();
  const refsQuery = useRead(
    'getGitRepositoryRefs',
    { id: repository.id },
    { meta: { suppressErrorToast: true } as any },
  );
  const refs = useMemo(() => refsQuery.data?.data.refs ?? [], [refsQuery.data?.data.refs]);
  const [resolvedCommit, setResolvedCommit] = useState<string | null>(commitSha ?? null);

  const selectedRef = useMemo(
    () => resolveSelectedRef(refs, branch ?? repository.defaultBranch),
    [branch, refs, repository.defaultBranch],
  );
  const effectiveBranch = branch ?? selectedRef?.branch ?? repository.defaultBranch ?? '';
  const activeCommit = commitSha ?? selectedRef?.resolvedCommitSha ?? resolvedCommit ?? undefined;
  const loadDirectory = useCallback(
    async (path: string, signal: AbortSignal) => {
      const response = await apiClient.api.listGitRepositoryDirectory(
        repository.id,
        { commitSha: activeCommit, path: path || undefined },
        { signal },
      );
      if (!activeCommit) setResolvedCommit(response.data.commitSha);

      return {
        entries: response.data.entries.map(mapRepositoryEntry),
        isTruncated: response.data.isTruncated,
      };
    },
    [activeCommit, apiClient, repository.id],
  );

  const providerRepositoryUrl = useMemo(() => getSafeHttpUrl(repository.url), [repository.url]);

  return (
    <div className={cn('flex min-h-[520px] flex-col overflow-hidden rounded-md border bg-background', className)}>
      {showToolbar && (
        <div className="flex flex-wrap items-center gap-2 border-b p-3">
          <div className="flex min-w-0 items-center gap-2">
            <GitBranch className="size-4 shrink-0 text-muted-foreground" />
            <span className="max-w-56 truncate text-sm" title={effectiveBranch || undefined}>
              {effectiveBranch || 'Branch unavailable'}
            </span>
          </div>
          <div className="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
            <GitCommitHorizontal className="size-3.5" />
            <code className="truncate">{shortSha(activeCommit)}</code>
          </div>
          {selectedRef?.lastSyncedAt && (
            <span className="hidden text-xs text-muted-foreground lg:inline">
              Synced {new Date(selectedRef.lastSyncedAt).toLocaleString()}
            </span>
          )}
          {providerRepositoryUrl && (
            <Button type="button" size="icon-sm" variant="ghost" className="ml-auto" asChild title="Open repository">
              <a href={providerRepositoryUrl} target="_blank" rel="noopener noreferrer">
                <ExternalLink className="size-4" />
              </a>
            </Button>
          )}
        </div>
      )}

      {!activeCommit && refsQuery.isLoading ? (
        <div className="flex flex-1 items-center justify-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="size-4 animate-spin" />
          Loading synchronized revisions
        </div>
      ) : !activeCommit && refs.length > 0 ? (
        <div className="p-4">
          <AlertMessage type="warning" title="Revision unavailable">
            The selected branch has not resolved to a commit.
          </AlertMessage>
        </div>
      ) : (
        <FileBrowser
          key={`${repository.id}:${activeCommit ?? 'default'}:${initialPath ?? ''}`}
          queryKey={['git-repository-browser', repository.id, activeCommit ?? 'default']}
          rootPath=""
          rootName={repository.name}
          loadDirectory={loadDirectory}
          initialPath={initialPath}
          highlightedPaths={highlightedPaths}
          renderPreview={(entry) => (
            <RepositoryFilePreview repositoryId={repository.id} commitSha={activeCommit} entry={entry} />
          )}
        />
      )}
    </div>
  );
}

function RepositoryFilePreview({
  repositoryId,
  commitSha,
  entry,
}: {
  repositoryId: string;
  commitSha?: string;
  entry: BrowserEntry | null;
}) {
  const query = useRead(
    'getGitRepositoryFileContent',
    entry ? { id: repositoryId, query: { commitSha, path: entry.path } } : undefined,
    {
      enabled: !!entry,
      gcTime: 15_000,
      meta: { suppressErrorToast: true } as any,
    },
  );

  if (!entry) {
    return <PreviewState>Select a file to preview it.</PreviewState>;
  }

  if (query.isLoading) {
    return (
      <PreviewState>
        <Loader2 className="size-4 animate-spin" />
        Loading {entry.name}
      </PreviewState>
    );
  }

  if (query.error) {
    const problem = (query.error as any)?.error;
    return (
      <div className="p-4">
        <AlertMessage type={problem?.status === 403 ? 'warning' : 'error'} title="File preview unavailable">
          {problem?.detail ?? problem?.title ?? query.error.message}
        </AlertMessage>
      </div>
    );
  }

  const file = query.data?.data;
  if (!file) return <PreviewState>File preview is unavailable.</PreviewState>;

  return (
    <div className="flex h-full min-h-[440px] flex-col">
      <div className="flex min-h-11 items-center gap-2 border-b px-3">
        <span className="min-w-0 flex-1 truncate text-sm font-medium" title={file.path}>
          {file.path}
        </span>
        {file.providerUrl && (
          <Button type="button" size="icon-sm" variant="ghost" asChild title="Open file in provider">
            <a href={file.providerUrl} target="_blank" rel="noopener noreferrer">
              <ExternalLink className="size-4" />
            </a>
          </Button>
        )}
      </div>
      <FileContent file={file} />
    </div>
  );
}

function FileContent({ file }: { file: GitFileContent }) {
  if (file.previewUnavailableReason || file.content == null) {
    return <PreviewState>{file.previewUnavailableReason ?? 'File preview is unavailable.'}</PreviewState>;
  }

  return (
    <div className="min-h-0 flex-1">
      <MonacoEditor
        value={file.content}
        filename={file.path}
        language={getRepositoryFileLanguage(file.path)}
        readOnly
        minimap={false}
        fillHeight
        className="h-full"
      />
    </div>
  );
}

function PreviewState({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex h-full min-h-72 items-center justify-center gap-2 p-6 text-sm text-muted-foreground">
      {children}
    </div>
  );
}

function mapRepositoryEntry(entry: {
  name: string;
  path: string;
  type: GitEntryTypeView;
  size?: number | string | null;
  targetCommitSha?: string | null;
}): BrowserEntry {
  return {
    name: entry.name,
    path: entry.path,
    type: mapRepositoryEntryType(entry.type),
    size: entry.size,
    secondaryText: entry.targetCommitSha ? shortSha(entry.targetCommitSha) : null,
  };
}

function mapRepositoryEntryType(type: GitEntryTypeView): BrowserEntry['type'] {
  switch (type) {
    case GitEntryTypeView.Directory:
      return 'directory';
    case GitEntryTypeView.Symlink:
      return 'symlink';
    case GitEntryTypeView.Submodule:
      return 'submodule';
    default:
      return 'file';
  }
}

function resolveSelectedRef(refs: GitRepositoryRefView[], branch?: string | null) {
  return branch ? refs.find((item) => item.branch === branch) : refs[0];
}

function shortSha(value?: string | null) {
  if (!value) return 'Unavailable';
  return value.slice(0, 12);
}

function getSafeHttpUrl(value: string) {
  try {
    const url = new URL(value);
    if (url.protocol !== 'http:' && url.protocol !== 'https:') return null;
    url.username = '';
    url.password = '';
    url.search = '';
    url.hash = '';
    url.pathname = url.pathname.replace(/\/+$/, '').replace(/\.git$/i, '');
    return url.toString();
  } catch {
    return null;
  }
}
