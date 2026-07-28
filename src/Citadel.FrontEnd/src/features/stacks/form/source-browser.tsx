import {
  GitChangedPathStatus,
  GitChangedPathView,
  GitRepositoryView,
  StackSpecGitStack,
  StackSource,
  StackView,
} from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { Button } from '@/components/ui/button';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { useRead } from '@/lib/hooks';
import { MonacoDiff } from '@/lib/monaco';
import { cn } from '@/lib/utils';
import { useQueryClient } from '@tanstack/react-query';
import { ArrowRight, FileDiff, FileText, GitBranch, GitCommitHorizontal, Loader2 } from 'lucide-react';
import { useMemo, useState } from 'react';
import { RepositoryBrowser } from '../../git-repos/browser/repository-browser';
import { getRepositoryFileLanguage } from '../../git-repos/browser/file-language';

type StackSourceFilesPanelProps = {
  stack: StackView;
  sourceDirty?: boolean;
};

export function StackSourceFilesPanel({ stack, sourceDirty }: StackSourceFilesPanelProps) {
  const source = stack.source?.sourceType === StackSource.Git ? stack.source : null;
  const persistedGitSpec = stack.stackSource === StackSource.Git ? (stack.spec as StackSpecGitStack | null) : null;
  const repositoryId = source?.gitRepositoryId ?? persistedGitSpec?.gitRepoId ?? '';
  const branch = source?.branch ?? persistedGitSpec?.branch ?? '';
  const queryClient = useQueryClient();
  const repositoryQuery = useRead(
    'getGitRepository',
    { id: repositoryId },
    {
      enabled: stack.stackSource === StackSource.Git && !!repositoryId,
      meta: { suppressErrorToast: true } as any,
    },
  );
  const refsQuery = useRead(
    'getGitRepositoryRefs',
    { id: repositoryId },
    {
      enabled: stack.stackSource === StackSource.Git && !!repositoryId,
      meta: { suppressErrorToast: true } as any,
    },
  );
  const [browserRevision, setBrowserRevision] = useState<'deployed' | 'latest' | null>(null);
  const [compareOpen, setCompareOpen] = useState(false);

  if (stack.stackSource !== StackSource.Git) return null;

  if (!repositoryId || !branch) {
    return (
      <section className="border-t pt-4">
        <h3 className="text-sm font-medium">Source files</h3>
        <p className="mt-1 text-sm text-muted-foreground">Save the Stack before browsing repository source.</p>
      </section>
    );
  }

  const repositoryProblem = (repositoryQuery.error as any)?.error;
  if (repositoryQuery.error) {
    return (
      <section className="border-t pt-4">
        <h3 className="mb-2 text-sm font-medium">Source files</h3>
        <AlertMessage type={repositoryProblem?.status === 403 ? 'warning' : 'error'}>
          {repositoryProblem?.status === 403
            ? 'You do not have permission to browse the linked repository.'
            : (repositoryProblem?.detail ?? 'The linked repository is unavailable.')}
        </AlertMessage>
      </section>
    );
  }

  const repository = repositoryQuery.data?.data;
  const latestCommit = refsQuery.data?.data.refs.find((item) => item.branch === branch)?.resolvedCommitSha ?? null;
  const deployedCommit = source?.resolvedCommitSha || null;
  const highlightedPaths = [...new Set([...(source?.composePaths ?? []), ...(source?.envFilePaths ?? [])])];
  const initialPath = source?.composePaths?.[0] ?? source?.envFilePaths?.[0] ?? null;
  const viewedCommit =
    browserRevision === 'latest' ? latestCommit : browserRevision === 'deployed' ? deployedCommit : null;

  const handleBrowserOpenChange = (open: boolean) => {
    if (open) return;

    clearRepositoryFileContentQueries(queryClient, repositoryId);
    setBrowserRevision(null);
  };

  const handleCompareOpenChange = (open: boolean) => {
    if (!open) clearRepositoryFileContentQueries(queryClient, repositoryId);
    setCompareOpen(open);
  };

  return (
    <section className="flex flex-col gap-3 border-t pt-4">
      <div>
        <h3 className="text-sm font-medium">Source files</h3>
        <p className="text-sm text-muted-foreground">Inspect the repository revision used by this Stack.</p>
      </div>

      {sourceDirty && <AlertMessage type="info">Save source changes to browse the updated configuration.</AlertMessage>}

      <div className="grid gap-x-8 gap-y-2 text-sm sm:grid-cols-2">
        <SourceValue
          icon={<GitBranch className="size-4" />}
          label="Repository"
          value={source?.gitRepositoryName ?? repository?.name ?? '-'}
        />
        <SourceValue icon={<GitBranch className="size-4" />} label="Branch" value={branch} />
        <SourceValue
          icon={<GitCommitHorizontal className="size-4" />}
          label="Deployed"
          value={shortSha(deployedCommit)}
          title={deployedCommit}
        />
        <SourceValue
          icon={<GitCommitHorizontal className="size-4" />}
          label="Latest synced"
          value={shortSha(latestCommit)}
          title={latestCommit}
        />
      </div>

      {highlightedPaths.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {highlightedPaths.map((path) => (
            <span key={path} className="rounded-sm bg-muted px-2 py-1 text-xs" title={path}>
              {path}
            </span>
          ))}
        </div>
      )}

      <div className="flex flex-wrap gap-2">
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={!repository || !deployedCommit}
          onClick={() => setBrowserRevision('deployed')}>
          Browse deployed source
        </Button>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={!repository || !latestCommit}
          onClick={() => setBrowserRevision('latest')}>
          Browse latest source
        </Button>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={!repository || !deployedCommit || !latestCommit || deployedCommit === latestCommit}
          onClick={() => setCompareOpen(true)}>
          <FileDiff className="size-4" />
          Compare
        </Button>
      </div>

      {deployedCommit && latestCommit && deployedCommit === latestCommit && (
        <p className="text-xs text-muted-foreground">Stack is using the latest synchronized commit.</p>
      )}

      {!deployedCommit && latestCommit && (
        <AlertMessage type="info">No deployed revision yet - showing latest synchronized revision.</AlertMessage>
      )}

      <Sheet open={browserRevision !== null} onOpenChange={handleBrowserOpenChange}>
        <SheetContent side="right" className="w-[min(96vw,1200px)] sm:max-w-none">
          <SheetHeader className="border-b">
            <SheetTitle>{stack.name} source</SheetTitle>
            <SheetDescription>
              {source?.gitRepositoryName ?? repository?.name} - {branch} - {shortSha(viewedCommit)}
            </SheetDescription>
          </SheetHeader>
          <div className="min-h-0 flex-1 overflow-auto px-4 pb-4">
            {repository && viewedCommit ? (
              <RepositoryBrowser
                repository={repository}
                branch={branch}
                commitSha={viewedCommit}
                initialPath={initialPath}
                highlightedPaths={highlightedPaths}
              />
            ) : (
              <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
                The selected revision is unavailable.
              </div>
            )}
          </div>
        </SheetContent>
      </Sheet>

      {repository && deployedCommit && latestCommit && (
        <StackSourceComparison
          open={compareOpen}
          onOpenChange={handleCompareOpenChange}
          repository={repository}
          baseCommitSha={deployedCommit}
          headCommitSha={latestCommit}
          highlightedPaths={highlightedPaths}
        />
      )}
    </section>
  );
}

function StackSourceComparison({
  open,
  onOpenChange,
  repository,
  baseCommitSha,
  headCommitSha,
  highlightedPaths,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  repository: GitRepositoryView;
  baseCommitSha: string;
  headCommitSha: string;
  highlightedPaths: readonly string[];
}) {
  const comparisonQuery = useRead(
    'compareGitRepositoryCommits',
    {
      id: repository.id,
      query: { baseCommitSha, headCommitSha },
    },
    {
      enabled: open,
      meta: { suppressErrorToast: true } as any,
    },
  );
  const files = useMemo(() => comparisonQuery.data?.data.files ?? [], [comparisonQuery.data?.data.files]);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const selected = files.find((file) => file.path === selectedPath) ?? files[0] ?? null;

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" className="w-[min(96vw,1200px)] sm:max-w-none">
        <SheetHeader className="border-b">
          <SheetTitle>Compare deployed with latest</SheetTitle>
          <SheetDescription>
            {shortSha(baseCommitSha)} <ArrowRight className="mx-1 inline size-3" /> {shortSha(headCommitSha)}
          </SheetDescription>
        </SheetHeader>
        <div className="grid min-h-0 flex-1 md:grid-cols-[320px_minmax(0,1fr)]">
          <div className="min-h-0 overflow-auto border-r p-2">
            {comparisonQuery.isLoading ? (
              <div className="flex items-center gap-2 p-3 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin" />
                Comparing commits
              </div>
            ) : comparisonQuery.error ? (
              <AlertMessage type="error">
                {(comparisonQuery.error as any)?.error?.detail ?? comparisonQuery.error.message}
              </AlertMessage>
            ) : files.length === 0 ? (
              <p className="p-3 text-sm text-muted-foreground">No changed files.</p>
            ) : (
              files.map((file) => (
                <button
                  key={`${file.status}:${file.previousPath ?? ''}:${file.path}`}
                  type="button"
                  className={cn(
                    'flex w-full min-w-0 items-center gap-2 rounded-sm px-2 py-2 text-left text-sm hover:bg-muted',
                    selected?.path === file.path && 'bg-muted',
                  )}
                  onClick={() => setSelectedPath(file.path)}>
                  <span className="w-4 shrink-0 font-mono text-xs text-muted-foreground">
                    {statusLabel(file.status)}
                  </span>
                  <span className="min-w-0 flex-1 truncate" title={file.path}>
                    {file.path}
                  </span>
                  {highlightedPaths.includes(file.path) && <FileText className="size-3.5 text-primary" />}
                </button>
              ))
            )}
          </div>
          <ComparisonPreview
            enabled={open}
            repositoryId={repository.id}
            baseCommitSha={baseCommitSha}
            headCommitSha={headCommitSha}
            file={selected}
          />
        </div>
      </SheetContent>
    </Sheet>
  );
}

function ComparisonPreview({
  repositoryId,
  baseCommitSha,
  headCommitSha,
  file,
  enabled,
}: {
  repositoryId: string;
  baseCommitSha: string;
  headCommitSha: string;
  file: GitChangedPathView | null;
  enabled: boolean;
}) {
  const basePath = file?.status === GitChangedPathStatus.Added ? null : (file?.previousPath ?? file?.path ?? null);
  const headPath = file?.status === GitChangedPathStatus.Deleted ? null : (file?.path ?? null);
  const baseQuery = useRead(
    'getGitRepositoryFileContent',
    basePath ? { id: repositoryId, query: { commitSha: baseCommitSha, path: basePath } } : undefined,
    { enabled: enabled && !!basePath, gcTime: 15_000, meta: { suppressErrorToast: true } as any },
  );
  const headQuery = useRead(
    'getGitRepositoryFileContent',
    headPath ? { id: repositoryId, query: { commitSha: headCommitSha, path: headPath } } : undefined,
    { enabled: enabled && !!headPath, gcTime: 15_000, meta: { suppressErrorToast: true } as any },
  );

  if (!file)
    return (
      <div className="flex items-center justify-center p-6 text-sm text-muted-foreground">Select a changed file.</div>
    );

  if (baseQuery.isLoading || headQuery.isLoading) {
    return (
      <div className="flex items-center justify-center gap-2 p-6 text-sm text-muted-foreground">
        <Loader2 className="size-4 animate-spin" />
        Loading file comparison
      </div>
    );
  }

  const base = basePath ? baseQuery.data?.data : null;
  const head = headPath ? headQuery.data?.data : null;
  const unavailable = base?.previewUnavailableReason ?? head?.previewUnavailableReason;
  if (baseQuery.error || headQuery.error || unavailable) {
    return (
      <div className="p-4">
        <AlertMessage type="warning" title="Text comparison unavailable">
          {unavailable ??
            (baseQuery.error as any)?.error?.detail ??
            (headQuery.error as any)?.error?.detail ??
            'The selected file cannot be compared as text.'}
        </AlertMessage>
      </div>
    );
  }

  return (
    <div className="min-h-0 overflow-auto p-4">
      <MonacoDiff
        original={base?.content ?? ''}
        modified={head?.content ?? ''}
        format="text"
        language={getRepositoryFileLanguage(file.path)}
        title={file.path}
      />
    </div>
  );
}

function SourceValue({
  icon,
  label,
  value,
  title,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  title?: string | null;
}) {
  return (
    <div className="grid min-w-0 grid-cols-[100px_minmax(0,1fr)] items-center gap-2">
      <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
        {icon}
        {label}
      </span>
      <span className="truncate" title={title ?? value}>
        {value}
      </span>
    </div>
  );
}

function shortSha(value?: string | null) {
  return value ? value.slice(0, 12) : 'Unavailable';
}

function statusLabel(status: GitChangedPathStatus) {
  switch (status) {
    case GitChangedPathStatus.Added:
      return 'A';
    case GitChangedPathStatus.Deleted:
      return 'D';
    case GitChangedPathStatus.Renamed:
      return 'R';
    case GitChangedPathStatus.Copied:
      return 'C';
    case GitChangedPathStatus.TypeChanged:
      return 'T';
    default:
      return 'M';
  }
}

function clearRepositoryFileContentQueries(queryClient: ReturnType<typeof useQueryClient>, repositoryId: string) {
  queryClient.removeQueries({
    predicate: (query) =>
      query.queryKey[0] === 'getGitRepositoryFileContent' &&
      (query.queryKey[1] as { id?: string } | undefined)?.id === repositoryId,
  });
}
