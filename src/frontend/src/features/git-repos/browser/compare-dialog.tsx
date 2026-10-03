import { useState } from 'react';
import { GitCompareArrows } from 'lucide-react';
import { GitChangedPath, GitChangedPathStatus, GitFileContent } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import Loader from '@/components/ui/loader';
import { useRead } from '@/lib/hooks';
import { MonacoDiff } from '@/lib/monaco';
import { requestErrorMessage } from '@/lib/request-error';
import { cn } from '@/lib/utils';
import { getRepositoryFileLanguage } from './file-language';

type ComparisonProps = {
  repositoryId: string;
  repositoryName?: string | null;
  baseCommitSha: string;
  headCommitSha: string;
  watchPaths?: string[] | null;
};

export function GitRepositoryCompareAction(props: ComparisonProps) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button type="button" variant="outline" size="sm" onClick={() => setOpen(true)}>
        <GitCompareArrows className="size-3.5" />
        View changes
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="flex h-[90dvh] w-[calc(100vw-1rem)] max-w-[1200px] flex-col gap-0 overflow-hidden p-0 sm:max-w-[1200px]">
          <DialogHeader className="border-b px-4 py-3 pr-12">
            <DialogTitle>Repository changes</DialogTitle>
            <DialogDescription>
              {props.repositoryName ?? 'Repository'} · Deployed {props.baseCommitSha.slice(0, 12)} → Available{' '}
              {props.headCommitSha.slice(0, 12)}
            </DialogDescription>
            {!!props.watchPaths?.length && (
              <p className="break-words text-xs text-muted-foreground">
                Stack watch paths: {props.watchPaths.join(', ')}. Showing all repository changes.
              </p>
            )}
          </DialogHeader>
          {open && <RepositoryComparison {...props} />}
        </DialogContent>
      </Dialog>
    </>
  );
}

function RepositoryComparison({ repositoryId, baseCommitSha, headCommitSha }: ComparisonProps) {
  const [selectedPath, setSelectedPath] = useState<string>();
  const query = useRead(
    'compareGitRepositoryCommits',
    {
      id: repositoryId,
      query: { baseCommitSha, headCommitSha },
    },
    { gcTime: 0, staleTime: Infinity, meta: { suppressErrorToast: true } },
  );
  const comparison = query.data?.data;
  const file = comparison?.files.find((item) => item.path === selectedPath) ?? comparison?.files[0];

  if (query.isLoading) return <Loader label="Loading changed files…" />;
  if (query.error)
    return (
      <div className="p-4">
        <AlertMessage type="warning" title="Comparison unavailable">
          {requestErrorMessage(query.error)}
        </AlertMessage>
      </div>
    );
  if (!comparison) return null;

  return (
    <>
      {comparison.isTruncated && (
        <div className="px-4 pt-3">
          <AlertMessage type="warning">
            The changed-file list reached the preview limit. Some files are not shown.
          </AlertMessage>
        </div>
      )}
      {!file ? (
        <p className="p-6 text-sm text-muted-foreground">No file changes between these commits.</p>
      ) : (
        <div className="flex min-h-0 flex-1 flex-col md:flex-row">
          <nav
            aria-label="Changed files"
            className="max-h-48 shrink-0 overflow-auto border-b md:max-h-none md:w-64 md:border-r md:border-b-0">
            <p className="px-3 py-2 text-xs text-muted-foreground">{comparison.files.length} changed files</p>
            {comparison.files.map((item) => (
              <button
                key={item.path}
                type="button"
                aria-current={item.path === file.path ? 'true' : undefined}
                onClick={() => setSelectedPath(item.path)}
                title={item.previousPath ? `${item.previousPath} → ${item.path}` : item.path}
                className={cn(
                  'flex w-full flex-col gap-1 border-l-2 border-transparent px-3 py-2 text-left text-xs hover:bg-muted/50 focus-visible:outline-ring',
                  item.path === file.path && 'border-l-primary bg-muted/50',
                )}>
                <span className="w-full truncate font-mono">{item.path}</span>
                <span className="text-muted-foreground">{item.status}</span>
              </button>
            ))}
          </nav>
          <div className="min-h-0 min-w-0 flex-1 overflow-auto p-3">
            <RepositoryFileDiff
              key={file.path}
              repositoryId={repositoryId}
              baseCommitSha={comparison.baseCommitSha}
              headCommitSha={comparison.headCommitSha}
              file={file}
            />
          </div>
        </div>
      )}
    </>
  );
}

function RepositoryFileDiff({
  repositoryId,
  baseCommitSha,
  headCommitSha,
  file,
}: ComparisonProps & { file: GitChangedPath }) {
  const hasOriginal = file.status !== GitChangedPathStatus.Added;
  const hasModified = file.status !== GitChangedPathStatus.Deleted;
  const original = useRead(
    'getGitRepositoryFileContent',
    {
      id: repositoryId,
      query: { commitSha: baseCommitSha, path: file.previousPath ?? file.path },
    },
    { enabled: hasOriginal, gcTime: 0, staleTime: Infinity, meta: { suppressErrorToast: true } },
  );
  const modified = useRead(
    'getGitRepositoryFileContent',
    {
      id: repositoryId,
      query: { commitSha: headCommitSha, path: file.path },
    },
    { enabled: hasModified, gcTime: 0, staleTime: Infinity, meta: { suppressErrorToast: true } },
  );
  if ((hasOriginal && original.isLoading) || (hasModified && modified.isLoading))
    return <Loader label="Loading diff…" />;
  const error = (hasOriginal && original.error) || (hasModified && modified.error);
  if (error)
    return (
      <AlertMessage type="warning" title="Diff unavailable">
        {requestErrorMessage(error)}
      </AlertMessage>
    );
  const before = original.data?.data;
  const after = modified.data?.data;
  const unavailable = (hasOriginal && previewUnavailable(before)) || (hasModified && previewUnavailable(after));
  if (unavailable)
    return (
      <AlertMessage type="info" title="Diff preview unavailable">
        {unavailable}
      </AlertMessage>
    );

  return (
    <>
      <div className="mb-3 flex flex-wrap gap-x-6 gap-y-1 text-xs text-muted-foreground">
        <span title={baseCommitSha}>Deployed: {file.previousPath ?? file.path}</span>
        <span title={headCommitSha}>Available: {file.path}</span>
      </div>
      <MonacoDiff
        original={hasOriginal ? before?.content : ''}
        modified={hasModified ? after?.content : ''}
        format="text"
        language={getRepositoryFileLanguage(file.path)}
        title={file.path}
      />
    </>
  );
}

function previewUnavailable(file?: GitFileContent): string | undefined {
  if (!file) return 'File content is unavailable.';
  if (file.previewUnavailableReason) return file.previewUnavailableReason;
  if (file.isBinary) return 'Binary files cannot be previewed.';
  if (file.isTruncated) return 'This file exceeds the preview limit.';
  if (file.content == null) return 'File content is unavailable.';
}
