import {
  PlatformView,
  StackConfigView,
  CreateStackInput,
  PatchStackInput,
  LookupResourceType,
  StackUpdateBehavior,
  StackSource,
  StackDriftMode,
  StackDriftPolicy,
  GitRepositoryRefView,
  GitComposeProjectCandidate,
} from '@/api/generated/api.types';
import {
  FormShell,
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldTextArea,
  ItemSelector,
  FieldSwitch,
  FieldSelect,
} from '@/components/custom/form-builder';
import { useState, useMemo, useEffect, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { useParams } from 'react-router';
import { ResourceSelectorField } from '@/components/custom/common';
import { MonacoEditor, MonacoToArrayEditor, type MonacoDiagnostic } from '@/lib/monaco';
import { WebhookConfigField } from '@/components/custom/webhook-config-field';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { GitBranch, Loader2, Search } from 'lucide-react';
import { toast } from 'sonner';
import * as monaco from 'monaco-editor';
import { ResourceTagSelector } from '@/features/tags/components';

const update_behaviors = {
  [StackUpdateBehavior.Disabled]: {
    label: 'Disabled',
    description: 'Do not check for updates.',
  },
  [StackUpdateBehavior.Notify]: {
    label: 'Notify Only',
    description: 'Report new image or Git source updates, but do not redeploy.',
  },
  [StackUpdateBehavior.ServiceAutoDeploy]: {
    label: 'Auto Deploy Services',
    description: 'Redeploy changed image services. Git source updates redeploy the stack.',
  },
  [StackUpdateBehavior.StackAutoDeploy]: {
    label: 'Auto Deploy Stack',
    description: 'Redeploy the entire stack when image or Git source updates are found.',
  },
};

const stack_source = {
  [StackSource.WebEditor]: {
    label: 'Web Editor',
    description: 'Define and manage the stack configuration directly in the web editor.',
  },
  [StackSource.Git]: {
    label: 'Git',
    description: 'Sync the stack configuration from a Git repository and track changes through version control.',
  },
};

type StackInput = CreateStackInput | PatchStackInput;

const EMPTY_STACK_CONFIG = {} as StackConfigView;
const EMPTY_GIT_REFS: GitRepositoryRefView[] = [];
const EMPTY_COMPOSE_PROJECTS: GitComposeProjectCandidate[] = [];
const EMPTY_RESOURCE_BINDING_LOOKUP: { name: string }[] = [];

const DEFAULT_DRIFT_POLICY: StackDriftPolicy = {
  mode: StackDriftMode.DetectOnly,
  alertOnDrift: true,
  markDegraded: true,
  autoStartStoppedContainers: false,
  autoResumePausedContainers: false,
  removeExtraContainers: false,
};

const drift_modes = {
  [StackDriftMode.Disabled]: {
    label: 'Disabled',
    description: 'Do not check this stack for runtime drift.',
  },
  [StackDriftMode.DetectOnly]: {
    label: 'Detect only',
    description: 'Detect drift and leave remediation manual.',
  },
  [StackDriftMode.AutoFix]: {
    label: 'Auto-fix safe drift',
    description: 'Allow safe runtime fixes for stopped or paused containers.',
  },
};

const normalizeDriftPolicy = (policy?: Partial<StackDriftPolicy> | null): StackDriftPolicy => {
  const next = {
    ...DEFAULT_DRIFT_POLICY,
    ...(policy ?? {}),
  };

  if (next.mode === StackDriftMode.Disabled) {
    return {
      ...next,
      alertOnDrift: false,
      markDegraded: false,
      autoStartStoppedContainers: false,
      autoResumePausedContainers: false,
      removeExtraContainers: false,
    };
  }

  return next;
};

const specTypeForSource = (stackSource?: StackSource): 'Git' | 'WebEditor' | undefined =>
  stackSource === StackSource.Git ? 'Git' : stackSource === StackSource.WebEditor ? 'WebEditor' : undefined;

const normalizeGitPath = (value: string) => value.trim().replaceAll('\\', '/');

const shortSha = (value?: string | null) => (value ? value.slice(0, 12) : '-');

const validateGitPath = (value?: string | null, { requireFile = false }: { requireFile?: boolean } = {}) => {
  const path = value?.trim();
  if (!path) return null;

  if (path.startsWith('/') || /^[a-zA-Z]:[\\/]/.test(path)) return 'Use a path relative to the repository root.';
  if (path.split(/[\\/]+/).some((segment) => segment === '..')) return 'Path cannot leave the repository root.';
  if (path.includes('\0')) return 'Path contains an invalid character.';
  if (requireFile && path.endsWith('/')) return 'Path must point to a file, not a directory.';

  return null;
};

const validateGitPathList = (
  value: unknown,
  { required = false, requireFile = false }: { required?: boolean; requireFile?: boolean } = {},
) => {
  if (!Array.isArray(value)) return required ? 'At least one path is required.' : null;

  const paths = value.map((item) => String(item ?? '').trim()).filter(Boolean);
  if (required && paths.length === 0) return 'At least one path is required.';

  const duplicates = new Set<string>();
  for (const path of paths) {
    const error = validateGitPath(path, { requireFile });
    if (error) return `${path}: ${error}`;

    const normalized = normalizeGitPath(path).toLowerCase();
    if (duplicates.has(normalized)) return `${path}: duplicate path.`;
    duplicates.add(normalized);
  }

  return null;
};

const validateGitPathItem =
  ({ requireFile = false }: { requireFile?: boolean } = {}) =>
  (path: string) => {
    const error = validateGitPath(path, { requireFile });
    return error ? `${path}: ${error}` : null;
  };

const validateCommitSha = (value?: string | null) => {
  const commitSha = value?.trim();
  if (!commitSha) return null;

  return /^[0-9a-f]{7,40}$/i.test(commitSha) ? null : 'Use a 7 to 40 character Git commit SHA.';
};

const duplicatePathMessage = (path: string) => `${path}: duplicate path.`;

const composeVariablePattern = /\$\{([^}]+)\}/g;
const composeVariableNamePattern = /^([A-Za-z_][A-Za-z0-9_]*)(?:(:?[-+?]).*)?$/;

const shouldWarnForMissingVariable = (operator?: string) => !operator || operator.includes('?');

const getComposeVariableDiagnostics = (
  compose: string | undefined,
  configurationNames: string[],
): MonacoDiagnostic[] => {
  if (!compose?.trim()) return [];

  const knownNames = new Set(configurationNames);
  const diagnostics: MonacoDiagnostic[] = [];

  compose.split('\n').forEach((line, lineIndex) => {
    for (const match of line.matchAll(composeVariablePattern)) {
      const expression = match[1]?.trim() ?? '';
      const parsed = composeVariableNamePattern.exec(expression);
      const startColumn = (match.index ?? 0) + 1;
      const endColumn = startColumn + match[0].length;

      if (!parsed) {
        diagnostics.push({
          lineNumber: lineIndex + 1,
          startColumn,
          endColumn,
          severity: monaco.MarkerSeverity.Warning,
          message: `Unsupported Compose variable expression '${match[0]}'.`,
        });
        continue;
      }

      const [, name, operator] = parsed;
      if (!knownNames.has(name) && shouldWarnForMissingVariable(operator)) {
        diagnostics.push({
          lineNumber: lineIndex + 1,
          startColumn,
          endColumn,
          severity: monaco.MarkerSeverity.Warning,
          message: `${name} is not defined in stack or global variables.`,
        });
      }
    }
  });

  return diagnostics;
};

const validateDiscoveredPath = (path: string, discoveredPaths: string[], discoveryReady: boolean, label: string) => {
  if (!discoveryReady) return null;

  const allowed = new Set(discoveredPaths.map(normalizeGitPath));
  const normalized = normalizeGitPath(path);

  return allowed.has(normalized) ? null : `${path}: not found in discovered ${label}.`;
};

const validateDiscoveredPathList = (
  value: unknown,
  discoveredPaths: string[],
  discoveryReady: boolean,
  label: string,
) => {
  if (!discoveryReady || !Array.isArray(value)) return null;

  for (const path of value.map((item) => String(item ?? '').trim()).filter(Boolean)) {
    const error = validateDiscoveredPath(path, discoveredPaths, discoveryReady, label);
    if (error) return error;
  }

  return null;
};

const validateGitDiscoveredPathItem =
  ({
    requireFile = false,
    discoveredPaths,
    discoveryReady,
    label,
  }: {
    requireFile?: boolean;
    discoveredPaths: string[];
    discoveryReady: boolean;
    label: string;
  }) =>
  (path: string) => {
    const syntaxError = validateGitPathItem({ requireFile })(path);
    if (syntaxError) return syntaxError;

    return validateDiscoveredPath(path, discoveredPaths, discoveryReady, label);
  };

const normalizeDisabledWebhook = <T extends Partial<StackInput> | StackConfigView>(value: T): T => {
  const spec = (value as any)?.spec;
  if (!spec?.webhook || spec.webhook.enabled) return value;

  const { webhook: _webhook, ...specWithoutWebhook } = spec;
  return {
    ...value,
    spec: specWithoutWebhook,
  };
};

const toPatchStackInput = (patch: Partial<StackInput>, original: StackConfigView): PatchStackInput => {
  const normalizedPatch = normalizeDisabledWebhook(patch);
  const data: Partial<PatchStackInput> = {};

  if ('platformId' in normalizedPatch) data.platformId = normalizedPatch.platformId;
  if ('spec' in normalizedPatch && normalizedPatch.spec) {
    data.spec = {
      ...normalizedPatch.spec,
      $type:
        (normalizedPatch.spec as any).$type ?? (original.spec as any)?.$type ?? specTypeForSource(original.stackSource),
    } as PatchStackInput['spec'];
  }
  if ('driftPolicy' in normalizedPatch) data.driftPolicy = normalizedPatch.driftPolicy;

  return data as PatchStackInput;
};

const GitBranchField = ({
  repoId,
  value,
  disabled,
  refs,
  onChange,
}: {
  repoId?: string | null;
  value?: string | null;
  disabled?: boolean;
  refs: Array<{ branch: string; resolvedCommitSha?: string | null; lastError?: string | null }>;
  onChange: (branch: string) => void;
}) => {
  const options = useMemo(() => {
    const known = new Map<string, string>();
    refs.forEach((ref) => known.set(ref.branch, ref.branch));
    if (value && !known.has(value)) known.set(value, value);
    return [...known.values()].map((branch) => ({
      value: branch,
      label: (
        <span className="flex min-w-0 items-center gap-2">
          <GitBranch className="size-3.5 text-muted-foreground" />
          <span className="truncate">{branch}</span>
          {refs.find((ref) => ref.branch === branch)?.resolvedCommitSha ? (
            <span className="font-mono text-xs text-muted-foreground">
              {shortSha(refs.find((ref) => ref.branch === branch)?.resolvedCommitSha)}
            </span>
          ) : null}
        </span>
      ),
    }));
  }, [refs, value]);

  if (!repoId || options.length === 0) {
    return (
      <div className="flex flex-col gap-2">
        <FieldInput value={value!} onChange={onChange} disabled={disabled} placeholder="e.g. main" />
        <p className="text-xs text-muted-foreground">
          Select and sync a repository to populate branch refs. You can still type a branch manually.
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      <FieldSelect
        value={value ?? undefined}
        onChange={onChange}
        options={options}
        disabled={disabled}
        placeholder="Select branch"
      />
      {refs.find((ref) => ref.branch === value)?.lastError ? (
        <p className="text-xs text-destructive">{refs.find((ref) => ref.branch === value)?.lastError}</p>
      ) : null}
    </div>
  );
};

const GitSourceStateHint = ({
  currentCommitSha,
  latestCommitSha,
  pinnedCommitSha,
}: {
  currentCommitSha?: string | null;
  latestCommitSha?: string | null;
  pinnedCommitSha?: string | null;
}) => (
  <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
    <Badge variant="outline" className="rounded-sm font-mono">
      current {shortSha(currentCommitSha)}
    </Badge>
    <Badge variant="outline" className="rounded-sm font-mono">
      latest {shortSha(latestCommitSha)}
    </Badge>
    {pinnedCommitSha ? (
      <Badge variant="secondary" className="rounded-sm font-mono">
        pinned {shortSha(pinnedCommitSha)}
      </Badge>
    ) : null}
    {currentCommitSha && latestCommitSha && currentCommitSha !== latestCommitSha ? (
      <span>Remote branch has a newer synced commit.</span>
    ) : null}
  </div>
);

const GitDiscoveredPathsField = ({
  canDiscover,
  discoveredPaths,
  discoveryReady,
  discovering,
  discoveryError,
  disabled,
  title,
  unavailableMessage,
  emptyMessage,
  helperText,
  requiredMessage,
  completionItemDetail,
  validateItem,
  value,
  onRefresh,
  onChange,
}: {
  canDiscover: boolean;
  discoveredPaths: string[];
  discoveryReady: boolean;
  discovering: boolean;
  discoveryError?: Error | null;
  disabled?: boolean;
  title: string;
  unavailableMessage: string;
  emptyMessage: string;
  helperText: string;
  requiredMessage?: string;
  completionItemDetail: string;
  validateItem: (path: string) => string | null;
  value?: string[] | null;
  onRefresh: () => void;
  onChange: (paths: string[] | undefined) => void;
}) => {
  const [open, setOpen] = useState(false);
  const copyPath = async (path: string) => {
    await navigator.clipboard.writeText(path);
    toast.success(`Copied "${path}" to clipboard`);
    setOpen(false);
  };

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-start">
        <Popover
          open={open}
          onOpenChange={(nextOpen) => {
            setOpen(nextOpen);
            if (nextOpen && canDiscover && !discovering) {
              onRefresh();
            }
          }}>
          <PopoverTrigger asChild>
            <Button
              type="button"
              variant="outline"
              className="ml-2 rounded-none font-normal"
              disabled={!canDiscover || discovering}
              title={canDiscover ? title : unavailableMessage}>
              {discovering ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <Search className="size-3.5 text-muted-foreground" />
              )}
              <span className="text-xs">Discover Paths</span>
            </Button>
          </PopoverTrigger>
          <PopoverContent align="start" className="w-80 p-1 bg-background">
            {!canDiscover ? <p className="px-2 py-2 text-sm text-muted-foreground">{unavailableMessage}</p> : null}

            {discoveryError ? <p className="px-2 py-2 text-sm text-destructive">{discoveryError.message}</p> : null}

            {discoveredPaths.length > 0 ? (
              <div className="max-h-72 overflow-y-auto">
                {discoveredPaths.map((path) => (
                  <button
                    key={path}
                    type="button"
                    disabled={disabled}
                    onClick={() => copyPath(path)}
                    className="flex w-full items-center justify-between gap-2 rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent disabled:pointer-events-none disabled:opacity-60">
                    <span className="truncate font-mono text-xs">{path}</span>
                  </button>
                ))}
              </div>
            ) : discoveryReady ? (
              <p className="px-2 py-2 text-sm text-muted-foreground">{emptyMessage}</p>
            ) : !discovering ? (
              <p className="px-2 py-2 text-sm text-muted-foreground">Open search to scan the selected branch.</p>
            ) : null}
          </PopoverContent>
        </Popover>
      </div>

      <MonacoToArrayEditor
        value={value!}
        helperText={helperText}
        language="string_list"
        unique
        duplicateMessage={duplicatePathMessage}
        requiredMessage={requiredMessage}
        validateItem={validateItem}
        completionItems={discoveredPaths}
        completionItemDetail={completionItemDetail}
        onChange={onChange}
      />
    </div>
  );
};

export const StackForm = ({
  mode,
  metadataChanged,
  disabled,
}: {
  mode: 'add' | 'edit';
  metadataChanged?: boolean;
  disabled?: boolean;
}) => {
  const id = useParams().id;
  const [update, setUpdate] = useState<Partial<StackInput>>({});
  const queryClient = useQueryClient();

  const { mutateAsync: createStack } = useMutate('createStack');
  const { mutateAsync: updateStack } = useMutate('updateStack');
  const { data: stackCfg } = useRead('getStackConfig', { stackId: id });
  const { data: stackViewData } = useRead('getStack', { stackId: id }, { enabled: mode === 'edit' && !!id });
  const { data: resourceBindingLookupData } = useRead('lookup', {
    query: {
      TargetResourceType: LookupResourceType.ResourceBinding,
      SourceResourceType: mode === 'edit' ? LookupResourceType.Stack : undefined,
      SourceResourceId: mode === 'edit' ? id : undefined,
    },
  });

  const resource: StackConfigView | undefined = stackCfg?.data;
  const stackView = stackViewData?.data;
  const original = resource ?? EMPTY_STACK_CONFIG;
  const formOriginal = useMemo(() => normalizeDisabledWebhook(original), [original]);
  const formUpdate = useMemo(() => normalizeDisabledWebhook(update), [update]);
  const currentStackSource = (update as Partial<CreateStackInput>).stackSource ?? original.stackSource;
  const currentGitRepoId = (update as any)?.spec?.gitRepoId ?? (original.spec as any)?.gitRepoId ?? null;
  const currentGitBranch = (update as any)?.spec?.branch ?? (original.spec as any)?.branch ?? null;
  const currentPinnedCommit = (update as any)?.spec?.commitSha ?? (original.spec as any)?.commitSha ?? null;
  const { data: gitRefsData } = useRead(
    'getGitRepositoryRefs',
    { id: currentGitRepoId ?? '' },
    { enabled: currentStackSource === StackSource.Git && !!currentGitRepoId },
  );
  const gitRefs = gitRefsData?.data.refs ?? EMPTY_GIT_REFS;
  const selectedBranchRef = gitRefs.find((ref) => ref.branch === currentGitBranch);
  const canDiscoverGitPaths = currentStackSource === StackSource.Git && !!currentGitRepoId && !!currentGitBranch;
  const {
    data: gitComposeDiscoveryData,
    isFetching: isDiscoveringGitPaths,
    isSuccess: gitComposeDiscoveryReady,
    error: gitComposeDiscoveryError,
    refetch: refreshGitComposeDiscovery,
  } = useRead(
    'discoverGitRepositoryComposeProjects',
    { id: currentGitRepoId ?? '', query: { branch: currentGitBranch ?? undefined } },
    { enabled: canDiscoverGitPaths },
  );
  const gitComposeProjects = gitComposeDiscoveryData?.data.projects ?? EMPTY_COMPOSE_PROJECTS;
  const discoveredComposePaths = useMemo(
    () => [...new Set(gitComposeProjects.flatMap((project) => project.composePaths.map(normalizeGitPath)))],
    [gitComposeProjects],
  );
  const discoveredEnvFilePaths = useMemo(
    () => [...new Set(gitComposeProjects.flatMap((project) => project.envFilePaths.map(normalizeGitPath)))],
    [gitComposeProjects],
  );
  const currentDriftPolicy = normalizeDriftPolicy({
    ...(original.driftPolicy ?? {}),
    ...((update as Partial<StackInput>).driftPolicy ?? {}),
  });
  const effectiveResourceBindings = resourceBindingLookupData?.data ?? EMPTY_RESOURCE_BINDING_LOOKUP;
  const effectiveConfigurationNames = useMemo(
    () => [...new Set(effectiveResourceBindings.map((entry) => entry.name))].sort(),
    [effectiveResourceBindings],
  );

  const refreshData = useCallback(() => {
    localStorage.removeItem(`stack:${id ?? 'new'}`);
    queryClient.invalidateQueries({ queryKey: ['getStackConfig', { stackId: id }] });
    queryClient.invalidateQueries({ queryKey: ['getStack', { stackId: id }] });
    queryClient.invalidateQueries({ queryKey: ['getStackDrift', { stackId: id }] });
  }, [id, queryClient]);

  useEffect(() => {
    if (!metadataChanged) return;
    refreshData();
  }, [metadataChanged, refreshData]);

  const { save: handleSave, isPending } = useSaveResource<StackInput, any>({
    mode,
    basePath: 'stacks',
    entityName: 'Stack',
    onCreate: (payload) => createStack({ data: payload as CreateStackInput }),
    onUpdate: () =>
      updateStack({
        id,
        data: toPatchStackInput(update, original),
      }),
    onRefresh: refreshData,
  });

  const patchDriftPolicy = useCallback(
    (prev: Partial<StackInput>, patch: Partial<StackDriftPolicy>): Partial<StackInput> => ({
      driftPolicy: normalizeDriftPolicy({
        ...(original.driftPolicy ?? {}),
        ...(prev.driftPolicy ?? {}),
        ...patch,
      }),
    }),
    [original.driftPolicy],
  );

  const schema = useMemo(
    () => ({
      general: defineSection<StackInput>({
        title: '',
        items: [
          ...(mode === 'add'
            ? [
                defineGroupField<StackInput>({
                  id: 'details',
                  label: 'Details',
                  items: [
                    defineField({
                      key: 'name',
                      label: 'Name',
                      required: true,
                      description: 'Internal identifier for this workload.',
                      validate: (v) => (!v ? 'Name is required' : null),
                      render: (val, set) => (
                        <FieldInput value={val} onChange={(v) => set({ name: v })} placeholder="stack-name" />
                      ),
                    }),
                    defineField({
                      key: 'description',
                      label: 'Description',
                      required: false,
                      description: 'Optional description of this workload.',
                      render: (val, set) => <FieldTextArea value={val} onChange={(v) => set({ description: v })} />,
                    }),
                    defineField({
                      key: 'tagIds',
                      label: 'Tags',
                      required: false,
                      description: 'Optional tags for filtering and grouping this stack.',
                      render: (val, set) => (
                        <ResourceTagSelector
                          value={val}
                          disabled={disabled}
                          onChange={(tagIds) => set({ tagIds })}
                        />
                      ),
                    }),
                  ],
                }),
              ]
            : []),
          defineField({
            key: 'platformId',
            label: 'Platform',
            required: true,
            disabled: false,
            description: 'Select the platform to deploy on.',
            render: (value, set) => {
              return (
                <ResourceSelectorField
                  sourceType={LookupResourceType.Stack}
                  targetType={LookupResourceType.Platform}
                  sourceResourceId={id}
                  selected={value}
                  onSelect={(v: PlatformView | undefined) => set({ platformId: v?.id })}
                  placeholder="Select Platform"
                />
              );
            },
          }),
          defineField({
            label: 'Stack Source',
            key: 'stackSource',
            description: 'Select where the stack configuration is managed.',
            required: true,
            disabled: !!currentStackSource,
            validate: (v) => (!v ? 'Source is required' : null),
            render: (val, set) => {
              return (
                <ItemSelector
                  collection={stack_source}
                  value={val}
                  disabled={disabled || !!currentStackSource}
                  onChange={(v: StackSource) => set({ stackSource: v })}
                />
              );
            },
          }),

          ...(currentStackSource === StackSource.Git
            ? [
                defineGroupField<StackInput>({
                  id: 'git_stack_source',
                  label: 'Git Stack',
                  title: 'Git Stack',
                  description:
                    'Select a repository branch and the Compose files that define this stack. Use one stack per Compose project in a monorepo.',
                  items: [
                    defineField({
                      key: 'spec.gitRepoId',
                      label: 'Repository',
                      required: true,
                      description: 'Repository that contains the Compose project for this stack.',
                      validate: (v) => (!v ? 'Repository is required' : null),
                      render: (value, set) => (
                        <ResourceSelectorField
                          sourceType={LookupResourceType.Stack}
                          targetType={LookupResourceType.GitRepository}
                          sourceResourceId={id}
                          selected={value}
                          onSelect={(v: { id: string } | undefined) =>
                            set((prev) => ({
                              spec: {
                                ...(prev.spec as any),
                                $type: 'Git',
                                gitRepoId: v?.id ?? '',
                                branch: undefined,
                              } as any,
                            }))
                          }
                          placeholder="Select Repository"
                        />
                      ),
                    }),
                    defineField({
                      key: 'spec.branch',
                      label: 'Branch',
                      required: true,
                      description:
                        'Branch to fetch, discover paths from, and track for updates. Required even when a commit pin is set.',
                      validate: (v) => (!v ? 'Branch is required' : null),
                      render: (value, set) => (
                        <GitBranchField
                          repoId={currentGitRepoId}
                          value={value}
                          refs={gitRefs}
                          disabled={disabled}
                          onChange={(v) =>
                            set((prev) => ({
                              spec: {
                                ...(prev.spec as any),
                                $type: 'Git',
                                branch: v,
                              } as any,
                            }))
                          }
                        />
                      ),
                    }),
                    defineField({
                      key: 'spec.commitSha',
                      label: 'Pin Commit SHA',
                      description:
                        'Optional commit from the selected branch. When set, deploys stay on this commit and Git push webhooks are ignored.',
                      validate: validateCommitSha,
                      render: (value, set) => (
                        <div className="flex flex-col gap-2">
                          <FieldInput
                            value={value}
                            onChange={(v) =>
                              set((prev) => ({
                                spec: {
                                  ...(prev.spec as any),
                                  $type: 'Git',
                                  commitSha: v || null,
                                } as any,
                              }))
                            }
                            placeholder="Optional commit SHA"
                          />
                          {value ? (
                            <p className="text-xs text-muted-foreground">
                              This stack will reapply this commit until the pin is removed or changed.
                            </p>
                          ) : null}
                          <GitSourceStateHint
                            currentCommitSha={stackView?.source?.resolvedCommitSha}
                            latestCommitSha={selectedBranchRef?.resolvedCommitSha}
                            pinnedCommitSha={currentPinnedCommit}
                          />
                        </div>
                      ),
                    }),
                    defineField({
                      key: 'spec.composePaths',
                      label: 'Compose Paths',
                      required: true,
                      description:
                        'Compose files relative to the repository root. Order matters: base file first, overrides after.',
                      validate: (v) =>
                        validateGitPathList(v, { required: true, requireFile: true }) ??
                        validateDiscoveredPathList(
                          v,
                          discoveredComposePaths,
                          gitComposeDiscoveryReady,
                          'compose files',
                        ),
                      hideValidationMessage: true,
                      render: (value, set) => (
                        <GitDiscoveredPathsField
                          canDiscover={canDiscoverGitPaths}
                          discoveredPaths={discoveredComposePaths}
                          discoveryReady={gitComposeDiscoveryReady}
                          discovering={isDiscoveringGitPaths}
                          discoveryError={gitComposeDiscoveryError}
                          title="Discover compose paths"
                          unavailableMessage="Select a repository and branch first."
                          emptyMessage="No compose files were found on this branch."
                          helperText="# compose.yml"
                          requiredMessage="At least one path is required."
                          completionItemDetail="Discovered compose file"
                          validateItem={validateGitDiscoveredPathItem({
                            requireFile: true,
                            discoveredPaths: discoveredComposePaths,
                            discoveryReady: gitComposeDiscoveryReady,
                            label: 'compose files',
                          })}
                          disabled={disabled}
                          value={value}
                          onRefresh={refreshGitComposeDiscovery}
                          onChange={(v: string[] | undefined) =>
                            set((prev) => ({
                              spec: {
                                ...(prev.spec as any),
                                $type: 'Git',
                                composePaths: (v ?? []).map(normalizeGitPath).filter(Boolean),
                              } as any,
                            }))
                          }
                        />
                      ),
                    }),
                    defineField({
                      key: 'spec.composeEnvFilesFromRepo',
                      label: 'Compose Env Files',
                      description:
                        'Optional env files from the Git snapshot, passed to Docker Compose before Citadel-generated variables. Values from the Variables tab override duplicate keys.',
                      validate: (v) =>
                        validateGitPathList(v, { requireFile: true }) ??
                        validateDiscoveredPathList(v, discoveredEnvFilePaths, gitComposeDiscoveryReady, 'env files'),
                      hideValidationMessage: true,
                      render: (value, set) => (
                        <GitDiscoveredPathsField
                          canDiscover={canDiscoverGitPaths}
                          discoveredPaths={discoveredEnvFilePaths}
                          discoveryReady={gitComposeDiscoveryReady}
                          discovering={isDiscoveringGitPaths}
                          discoveryError={gitComposeDiscoveryError}
                          title="Discover env files"
                          unavailableMessage="Select a repository and branch first."
                          emptyMessage="No env files were found on this branch."
                          value={value ?? (original.spec as any)?.additionalEnvFileFromRepo}
                          helperText="# .env"
                          completionItemDetail="Discovered env file"
                          validateItem={validateGitDiscoveredPathItem({
                            requireFile: true,
                            discoveredPaths: discoveredEnvFilePaths,
                            discoveryReady: gitComposeDiscoveryReady,
                            label: 'env files',
                          })}
                          disabled={disabled}
                          onRefresh={refreshGitComposeDiscovery}
                          onChange={(v: string[] | undefined) =>
                            set((prev) => ({
                              spec: {
                                ...(prev.spec as any),
                                $type: 'Git',
                                composeEnvFilesFromRepo: (v ?? []).map(normalizeGitPath).filter(Boolean),
                              } as any,
                            }))
                          }
                        />
                      ),
                    }),
                    defineField<StackInput, 'spec.envFilePath'>({
                      key: 'spec.envFilePath',
                      label: 'Generated Env File Path',
                      description:
                        'Optional path, relative to the Docker Compose run directory, where Citadel writes resolved Variables tab entries before deploy. Leave empty for the default temporary path.',
                      render: (value, set) => (
                        <FieldInput
                          value={value}
                          onChange={(v) =>
                            set((prev) => ({
                              spec: {
                                ...(prev.spec as any),
                                $type: 'Git',
                                envFilePath: v || null,
                              } as any,
                            }))
                          }
                          placeholder="Default generated path"
                        />
                      ),
                    }),
                  ],
                }),
              ]
            : currentStackSource === StackSource.WebEditor
              ? [
                  defineGroupField<StackInput>({
                    id: 'manual_stack_source',
                    label: 'Compose File',
                    items: [
                      defineField({
                        key: 'spec.composeFile',
                        label: 'Compose File',
                        required: true,
                        description: 'Define your docker compose YAML content.',
                        validate: (v) => (!v ? 'Compose file is required' : null),
                        render: (value, set) => (
                          <MonacoEditor
                            language="yaml"
                            filename="compose.yaml"
                            value={value ?? DEFAULT_STACK_FILE_CONTENTS}
                            diagnostics={getComposeVariableDiagnostics(
                              value ?? DEFAULT_STACK_FILE_CONTENTS,
                              effectiveConfigurationNames,
                            )}
                            completionItems={effectiveConfigurationNames}
                            completionItemDetail="Citadel variable or secret"
                            completionMode="variable"
                            onValueChange={(v) =>
                              set((prev) => ({
                                spec: {
                                  ...(prev.spec as any),
                                  $type: 'WebEditor',
                                  composeFile: v,
                                } as any,
                              }))
                            }
                          />
                        ),
                      }),
                      defineField<StackInput, 'spec.envFilePath'>({
                        key: 'spec.envFilePath',
                        label: 'Generated Env File Path',
                        description:
                          'Optional path, relative to the Docker Compose run directory, where Citadel writes resolved Variables tab entries before deploy. Leave empty for the default temporary path.',
                        render: (value, set) => (
                          <FieldInput
                            value={value}
                            onChange={(v) =>
                              set((prev) => ({
                                spec: {
                                  ...(prev.spec as any),
                                  $type: 'WebEditor',
                                  envFilePath: v || null,
                                } as any,
                              }))
                            }
                            placeholder="Default generated path"
                          />
                        ),
                      }),
                    ],
                  }),
                ]
              : []),

          ...(currentStackSource
            ? [
                defineField<StackInput, 'spec.registryId'>({
                  key: 'spec.registryId',
                  label: 'Registry',
                  required: true,
                  description: 'Select the registry to pull the images from.',
                  render: (val, set) => {
                    return (
                      <ResourceSelectorField
                        sourceType={LookupResourceType.Stack}
                        targetType={LookupResourceType.Registry}
                        sourceResourceId={id}
                        selected={val}
                        onSelect={(v: any) =>
                          set((prev) => ({
                            spec: {
                              ...prev.spec!,
                              registryId: v.id,
                            },
                          }))
                        }
                        placeholder="Select Registry"
                        className="sm:min-w-100"
                      />
                    );
                  },
                }),
                defineField<StackInput, 'spec.updateBehavior'>({
                  key: 'spec.updateBehavior',
                  label: 'Auto Update',
                  description: 'Choose how the platform handles new stack versions when they become available.',
                  render: (value, set) => {
                    return (
                      <div className="flex flex-col gap-2">
                        <ItemSelector
                          collection={update_behaviors}
                          value={value}
                          disabled={disabled}
                          onChange={(updateBehavior: StackUpdateBehavior) => {
                            set((prev) => ({
                              spec: {
                                ...prev.spec!,
                                updateBehavior: updateBehavior,
                              },
                            }));
                          }}
                        />
                      </div>
                    );
                  },
                }),
              ]
            : []),
        ],
      }),
      ...(currentStackSource
        ? {
            Advanced: defineSection<StackInput>({
              title: 'Advanced',
              items: [
                defineField<StackInput, 'spec.projectName'>({
                  key: 'spec.projectName',
                  label: 'Project Name',
                  description: 'Optional Docker Compose project name override.',
                  render: (value, set) => (
                    <FieldInput
                      value={value}
                      onChange={(v) =>
                        set((prev) => ({
                          spec: {
                            ...prev.spec!,
                            projectName: v || null,
                          },
                        }))
                      }
                      placeholder="Optional project name"
                    />
                  ),
                }),
                ...(currentStackSource === StackSource.Git
                  ? [
                      defineGroupField<StackInput>({
                        id: 'git_stack_paths',
                        label: 'Git Source Paths',
                        title: 'Git Source Paths',
                        description:
                          'Advanced path controls for monorepos. Defaults work for most stacks when compose files live beside their assets.',
                        items: [
                          defineField<StackInput, 'spec.workingDirectory'>({
                            key: 'spec.workingDirectory',
                            label: 'Working Directory',
                            description:
                              'Repository path used as the Docker Compose project directory. Defaults to the first compose file folder.',
                            validate: (v) => validateGitPath(v),
                            render: (value, set) => (
                              <FieldInput
                                value={value}
                                onChange={(v) =>
                                  set((prev) => ({
                                    spec: {
                                      ...(prev.spec as any),
                                      $type: 'Git',
                                      workingDirectory: v ? normalizeGitPath(v) : null,
                                    } as any,
                                  }))
                                }
                                placeholder="e.g. stacks/mystack"
                              />
                            ),
                          }),
                          defineField<StackInput, 'spec.watchPaths'>({
                            key: 'spec.watchPaths',
                            label: 'Watch Paths',
                            description:
                              'Optional paths used to decide whether a commit affects this stack. Leave empty to watch the working directory, compose files, and repo env files.',
                            validate: (v) => validateGitPathList(v),
                            hideValidationMessage: true,
                            render: (value, set) => (
                              <MonacoToArrayEditor
                                value={value}
                                helperText="# stacks/mystack/**"
                                language="string_list"
                                unique
                                duplicateMessage={duplicatePathMessage}
                                validateItem={validateGitPathItem()}
                                onChange={(v: string[] | undefined) =>
                                  set((prev) => ({
                                    spec: {
                                      ...(prev.spec as any),
                                      $type: 'Git',
                                      watchPaths: (v ?? []).map(normalizeGitPath).filter(Boolean),
                                    } as any,
                                  }))
                                }
                              />
                            ),
                          }),
                        ],
                      }),
                      defineGroupField<StackInput>({
                        id: 'webhook',
                        label: 'Webhook',
                        title: 'Webhook',
                        description: 'Trigger a deploy from your Git provider when this branch receives a push.',
                        items: [
                          defineField<StackInput, 'spec.webhook'>({
                            key: 'spec.webhook',
                            label: 'Enabled',
                            render: (value, set) => (
                              <WebhookConfigField
                                resourceType="stack"
                                resourceId={id}
                                execution="deploy"
                                value={value ?? null}
                                defaultBranch={currentGitBranch}
                                disabled={disabled}
                                onChange={(webhook) =>
                                  set((prev) => ({
                                    spec: {
                                      ...(original.spec as any),
                                      ...(prev.spec as any),
                                      $type: 'Git',
                                      webhook,
                                    } as any,
                                  }))
                                }
                              />
                            ),
                          }),
                        ],
                      }),
                    ]
                  : []),
                defineGroupField<StackInput>({
                  id: 'drift_policy',
                  label: 'Drift Management',
                  title: 'Drift Management',
                  description:
                    'Detect runtime differences between the compose file and the containers currently running on the platform.',
                  items: [
                    defineField({
                      key: 'driftPolicy.mode',
                      label: 'Mode',
                      description: 'Choose how this stack handles drift checks.',
                      render: (value, set) => (
                        <ItemSelector
                          collection={drift_modes}
                          value={value ?? StackDriftMode.DetectOnly}
                          disabled={disabled}
                          onChange={(mode: StackDriftMode) => set((prev) => patchDriftPolicy(prev, { mode }))}
                        />
                      ),
                    }),
                    ...(currentDriftPolicy.mode !== StackDriftMode.Disabled
                      ? [
                          defineField<StackInput, 'driftPolicy.alertOnDrift'>({
                            key: 'driftPolicy.alertOnDrift',
                            label: 'Alert On Drift',
                            description: 'Emit a StackDriftDetected alert when drift is detected.',
                            render: (value, set) => (
                              <FieldSwitch
                                id="stack-drift-alert-on-drift"
                                checked={value ?? currentDriftPolicy.alertOnDrift}
                                disabled={disabled}
                                onChange={(alertOnDrift) => set((prev) => patchDriftPolicy(prev, { alertOnDrift }))}
                              />
                            ),
                          }),
                          defineField<StackInput, 'driftPolicy.markDegraded'>({
                            key: 'driftPolicy.markDegraded',
                            label: 'Mark Degraded',
                            description: 'Mark the stack degraded and record an activity event when drift is detected.',
                            render: (value, set) => (
                              <FieldSwitch
                                id="stack-drift-mark-degraded"
                                checked={value ?? currentDriftPolicy.markDegraded}
                                disabled={disabled}
                                onChange={(markDegraded) => set((prev) => patchDriftPolicy(prev, { markDegraded }))}
                              />
                            ),
                          }),
                        ]
                      : []),
                    ...(currentDriftPolicy.mode === StackDriftMode.AutoFix
                      ? [
                          defineField<StackInput, 'driftPolicy.autoStartStoppedContainers'>({
                            key: 'driftPolicy.autoStartStoppedContainers',
                            label: 'Auto Start Stopped Containers',
                            description:
                              'When auto-fix is enabled, start containers that belong to this stack but are stopped.',
                            render: (value, set) => (
                              <FieldSwitch
                                id="stack-drift-auto-start"
                                checked={value ?? currentDriftPolicy.autoStartStoppedContainers}
                                disabled={disabled}
                                onChange={(autoStartStoppedContainers) =>
                                  set((prev) => patchDriftPolicy(prev, { autoStartStoppedContainers }))
                                }
                              />
                            ),
                          }),
                          defineField<StackInput, 'driftPolicy.autoResumePausedContainers'>({
                            key: 'driftPolicy.autoResumePausedContainers',
                            label: 'Auto Resume Paused Containers',
                            description:
                              'When auto-fix is enabled, resume containers that belong to this stack but are paused.',
                            render: (value, set) => (
                              <FieldSwitch
                                id="stack-drift-auto-resume"
                                checked={value ?? currentDriftPolicy.autoResumePausedContainers}
                                disabled={disabled}
                                onChange={(autoResumePausedContainers) =>
                                  set((prev) => patchDriftPolicy(prev, { autoResumePausedContainers }))
                                }
                              />
                            ),
                          }),
                          defineField<StackInput, 'driftPolicy.removeExtraContainers'>({
                            key: 'driftPolicy.removeExtraContainers',
                            label: 'Remove Extra Containers',
                            description:
                              'Reserved for destructive cleanup. It stays off unless explicitly enabled for auto-fix.',
                            render: (value, set) => (
                              <FieldSwitch
                                id="stack-drift-remove-extra"
                                checked={value ?? currentDriftPolicy.removeExtraContainers}
                                disabled={disabled}
                                onChange={(removeExtraContainers) =>
                                  set((prev) => patchDriftPolicy(prev, { removeExtraContainers }))
                                }
                              />
                            ),
                          }),
                        ]
                      : []),
                  ],
                }),
                defineGroupField<StackInput>({
                  id: 'spec.preDeploy',
                  label: 'Pre Deploy',
                  title: 'Pre Deploy',
                  description:
                    "Execute a shell command before running docker compose up. The 'path' is relative to the Run Directory",
                  items: [
                    defineField({
                      key: 'spec.preDeploy.path',
                      label: 'Path',
                      render: (val, set) => (
                        <FieldInput
                          value={val}
                          onChange={(v) =>
                            set((prev) => ({
                              spec: {
                                ...prev.spec!,
                                preDeploy: {
                                  ...(prev.spec?.preDeploy ?? {}),
                                  path: v,
                                  commands: prev.spec?.preDeploy?.commands ?? [],
                                },
                              },
                            }))
                          }
                          placeholder="Command working directory"
                        />
                      ),
                    }),
                    defineField({
                      key: 'spec.preDeploy.commands',
                      label: 'Commands',
                      required: false,
                      render: (value, set) => (
                        <MonacoToArrayEditor
                          value={value}
                          helperText="# Add multiple commands on new lines"
                          language="string_list"
                          onChange={(v: string[] | undefined) =>
                            set((prev) => ({
                              spec: {
                                ...prev.spec!,
                                preDeploy: {
                                  ...(prev.spec?.preDeploy ?? {}),
                                  commands: v ?? [],
                                },
                              },
                            }))
                          }
                        />
                      ),
                    }),
                  ],
                }),
                defineGroupField<StackInput>({
                  id: 'spec.postDeploy',
                  label: 'Post Deploy',
                  title: 'Post Deploy',
                  description:
                    "Execute a shell command after running docker compose up. The 'path' is relative to the Run Directory",
                  items: [
                    defineField({
                      key: 'spec.postDeploy.path',
                      label: 'Path',
                      render: (val, set) => (
                        <FieldInput
                          value={val}
                          onChange={(v) =>
                            set((prev) => ({
                              spec: {
                                ...prev.spec!,
                                postDeploy: {
                                  ...(prev.spec?.postDeploy ?? {}),
                                  path: v,
                                  commands: prev.spec?.postDeploy?.commands ?? [],
                                },
                              },
                            }))
                          }
                          placeholder="Command working directory"
                        />
                      ),
                    }),
                    defineField({
                      key: 'spec.postDeploy.commands',
                      label: 'Commands',
                      required: false,
                      render: (value, set) => (
                        <MonacoToArrayEditor
                          value={value}
                          helperText="# Add multiple commands on new lines"
                          language="string_list"
                          onChange={(v: string[] | undefined) =>
                            set((prev) => ({
                              spec: {
                                ...prev.spec!,
                                postDeploy: {
                                  ...(prev.spec?.postDeploy ?? {}),
                                  commands: v ?? [],
                                },
                              },
                            }))
                          }
                        />
                      ),
                    }),
                  ],
                }),

                defineField({
                  key: 'spec.destroyBeforeDeploy',
                  label: 'Destroy',
                  description: `Ensure 'docker compose down' is run before redeploying the Stack.`,
                  required: false,
                  render: (value, set) => (
                    <FieldSwitch
                      checked={value ?? true}
                      id="spec.destroyBeforeDeploy"
                      onChange={(value) =>
                        set((prev) => ({
                          spec: {
                            ...prev.spec!,
                            destroyBeforeDeploy: value,
                          },
                        }))
                      }
                    />
                  ),
                }),
              ],
            }),
          }
        : {}),
    }),
    [
      disabled,
      mode,
      id,
      currentStackSource,
      currentGitRepoId,
      currentGitBranch,
      currentPinnedCommit,
      currentDriftPolicy,
      canDiscoverGitPaths,
      discoveredComposePaths,
      discoveredEnvFilePaths,
      gitComposeDiscoveryReady,
      gitComposeDiscoveryError,
      isDiscoveringGitPaths,
      refreshGitComposeDiscovery,
      patchDriftPolicy,
      original.spec,
      gitRefs,
      selectedBranchRef?.resolvedCommitSha,
      stackView?.source?.resolvedCommitSha,
      effectiveConfigurationNames,
    ],
  );

  return (
    <FormShell
      mode={mode}
      schema={schema}
      original={formOriginal}
      update={formUpdate}
      setUpdate={setUpdate}
      onSave={handleSave}
      pending={isPending}
      disabled={disabled}
      draftKey={`stack:${id ?? 'new'}`}
      draftVersion={1}
    />
  );
};

const DEFAULT_STACK_FILE_CONTENTS = `## Add your compose file here
services:
  hello_world:
    image: hello-world
    # networks:
    #   - default
    # ports:
    #   - 3000:3000
    # volumes:
    #   - data:/data

# networks:
#   default: {}

# volumes:
#   data:
`;
