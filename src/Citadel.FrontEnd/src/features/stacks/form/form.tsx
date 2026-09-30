import { useFormDraftKey } from '@/lib/form-drafts';
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
  AuthorizedProject,
  StackBuildImageBinding,
  BuildRunStatus,
  LicenseCapability,
  AdoptionIssueSeverity,
  ImportRequest,
  StackSpec,
  ComposeProjectImportValidation,
  PlatformType,
  StackReleaseStatus,
  SwarmStackCompatibilityReport,
  SwarmStackCompatibilitySeverity,
  StackImportKind,
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
import { useState, useMemo, useEffect, useCallback, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { useParams, useSearchParams } from 'react-router';
import { ResourceSelectorField } from '@/components/custom/common';
import { MonacoEditor, MonacoToArrayEditor, type MonacoDiagnostic } from '@/lib/monaco';
import { WebhookConfigField } from '@/components/custom/webhook-config-field';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { FolderInput, GitBranch, KeyRound, Loader2, Plus, Search, Trash2 } from 'lucide-react';
import { toast } from 'sonner';
import { DiagnosticSeverity } from '@/lib/monaco/diagnostics';
import { ResourceTagSelector } from '@/features/tags/components';
import { AlertMessage } from '@/components/custom/alert-message';
import { BuildImageProvenanceStatus } from '@/features/builds/build-image-provenance-status';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import { getDriftModePreset } from './drift-policy';
import { ActionWithDialog } from '@/components/custom/action-with-dialog';
import { GitRepositoryBrowseAction } from '@/features/git-repos/browser/browser-dialog';
import { getSwarmComposeDiagnostics } from './swarm-compose-diagnostics';

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
const EMPTY_BUILD_PROJECTS: AuthorizedProject[] = [];

const DEFAULT_DRIFT_POLICY: StackDriftPolicy = {
  mode: StackDriftMode.Disabled,
  alertOnDrift: false,
  markDegraded: false,
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

const normalizeStackSpec = (spec: StackSpec, stackSource: StackSource): StackSpec => {
  const { $type, ...values } = spec as StackSpec & { $type?: 'Git' | 'WebEditor' };
  return {
    $type: $type ?? specTypeForSource(stackSource),
    ...values,
  } as StackSpec;
};

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
          severity: DiagnosticSeverity.Warning,
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
          severity: DiagnosticSeverity.Warning,
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
    data.spec = normalizeStackSpec(normalizedPatch.spec, original.stackSource) as PatchStackInput['spec'];
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

const StackBuildImageBindingsField = ({
  value,
  projects,
  isLoading,
  disabled,
  redeployEnabled,
  onChange,
}: {
  value?: StackBuildImageBinding[] | null;
  projects: AuthorizedProject[];
  isLoading?: boolean;
  disabled?: boolean;
  redeployEnabled: boolean;
  onChange: (value: StackBuildImageBinding[]) => void;
}) => {
  const bindings = value ?? [];
  const projectOptions = useMemo(
    () =>
      projects.map((project) => ({
        value: project.id,
        label: (
          <span className="flex min-w-0 flex-col">
            <span className="truncate">{project.name}</span>
            <span className="truncate text-xs text-muted-foreground">
              {project.imageRepository}:{project.branch}
            </span>
          </span>
        ),
      })),
    [projects],
  );

  const setBinding = (index: number, patch: Partial<StackBuildImageBinding>) => {
    const next = bindings.map((binding, currentIndex) =>
      currentIndex === index
        ? {
            ...binding,
            ...patch,
          }
        : binding,
    );
    onChange(next);
  };

  const removeBinding = (index: number) => {
    onChange(bindings.filter((_, currentIndex) => currentIndex !== index));
  };

  return (
    <div className="flex flex-col gap-3">
      {bindings.length === 0 ? (
        <p className="text-xs text-muted-foreground">
          Add a row only for Compose services whose image should come from a Citadel build.
        </p>
      ) : null}

      {bindings.map((binding, index) => {
        const hasProjectSelection = projectOptions.some((option) => option.value === binding.buildProjectId);
        const selectedProject = projects.find((project) => project.id === binding.buildProjectId);
        const latestRun = selectedProject?.latestRun;
        const latestImageReference = latestRun?.imageReferences?.[0];
        const hasSuccessfulImage = latestRun?.status === BuildRunStatus.Succeeded && !!latestImageReference;
        return (
          <div
            key={`${binding.serviceName}:${index}`}
            className="grid gap-3 border-b pb-3 last:border-b-0 md:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)_auto_auto]">
            <div className="flex min-w-0 flex-col gap-1">
              <span className="text-xs font-medium text-muted-foreground">Compose Service</span>
              <FieldInput
                value={binding.serviceName}
                disabled={disabled}
                onChange={(serviceName) => setBinding(index, { serviceName })}
                placeholder="e.g. api"
                className="w-full max-w-full"
              />
            </div>
            <div className="flex min-w-0 flex-col gap-1">
              <span className="text-xs font-medium text-muted-foreground">Build</span>
              <FieldSelect
                value={hasProjectSelection ? binding.buildProjectId : undefined}
                options={projectOptions}
                disabled={disabled || isLoading || projectOptions.length === 0}
                placeholder={isLoading ? 'Loading builds...' : 'Select build'}
                onChange={(buildProjectId) => setBinding(index, { buildProjectId })}
                className="w-full max-w-full"
              />
              <BuildBindingStatus
                binding={binding}
                buildProjectId={binding.buildProjectId}
                isLoading={isLoading}
                project={selectedProject}
                hasProjects={projectOptions.length > 0}
                hasSuccessfulImage={hasSuccessfulImage}
                latestStatus={latestRun?.status}
                latestImageReference={latestImageReference}
                latestDigest={latestRun?.imageDigest}
                latestBuildRunId={latestRun?.id}
              />
            </div>
            <div className="flex min-w-32 flex-col gap-1">
              <span className="text-xs font-medium text-muted-foreground">Redeploy</span>
              <label className="flex h-10 items-center gap-2 text-sm" htmlFor={`stack-build-redeploy-${index}`}>
                <FieldSwitch
                  id={`stack-build-redeploy-${index}`}
                  checked={binding.redeployOnBuild ?? false}
                  disabled={disabled || (!redeployEnabled && !binding.redeployOnBuild)}
                  onChange={(redeployOnBuild) => setBinding(index, { redeployOnBuild })}
                />
                <span className="text-muted-foreground">
                  {redeployEnabled || binding.redeployOnBuild ? 'On success' : 'Team'}
                </span>
              </label>
            </div>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="mt-5"
              disabled={disabled}
              onClick={() => removeBinding(index)}
              title="Remove build image binding">
              <Trash2 className="size-4" />
            </Button>
          </div>
        );
      })}

      <Button
        type="button"
        variant="outline"
        className="w-fit"
        disabled={disabled}
        onClick={() =>
          onChange([
            ...bindings,
            {
              serviceName: '',
              buildProjectId: '',
              redeployOnBuild: false,
            },
          ])
        }>
        <Plus className="size-4" />
        Add Service Binding
      </Button>
    </div>
  );
};

const BuildBindingStatus = ({
  binding,
  buildProjectId,
  isLoading,
  project,
  hasProjects,
  hasSuccessfulImage,
  latestStatus,
  latestImageReference,
  latestDigest,
  latestBuildRunId,
}: {
  binding: StackBuildImageBinding;
  buildProjectId?: string | null;
  isLoading?: boolean;
  project?: AuthorizedProject;
  hasProjects: boolean;
  hasSuccessfulImage: boolean;
  latestStatus?: BuildRunStatus;
  latestImageReference?: string;
  latestDigest?: string | null;
  latestBuildRunId?: string | null;
}) => {
  if (isLoading) return null;
  if (!hasProjects) return <p className="text-xs text-amber-600">Create a build project before mapping services.</p>;
  if (buildProjectId && !project)
    return <p className="text-xs text-amber-600">Selected build is no longer available.</p>;
  if (!project)
    return <p className="text-xs text-muted-foreground">Select the build that produces this service image.</p>;
  if (hasSuccessfulImage) {
    return (
      <BuildImageProvenanceStatus
        value={binding}
        latestImageReference={latestImageReference}
        latestDigest={latestDigest}
        latestBuildRunId={latestBuildRunId}
      />
    );
  }
  if (latestStatus) {
    return (
      <p className="text-xs text-amber-600">
        Latest run {latestStatus}; this service can deploy after a successful run.
      </p>
    );
  }
  return <p className="text-xs text-muted-foreground">No build image yet; run this build before applying the stack.</p>;
};

function validateStackBuildImageBindings(value?: StackBuildImageBinding[] | null): string | null {
  const bindings = value ?? [];
  const serviceNames = new Set<string>();

  for (const binding of bindings) {
    const serviceName = binding.serviceName?.trim();
    if (!serviceName) return 'Service name is required for each build image binding';
    if (!binding.buildProjectId) return `Build is required for service "${serviceName}"`;

    const normalizedServiceName = serviceName.toLowerCase();
    if (serviceNames.has(normalizedServiceName)) {
      return `Service "${serviceName}" is mapped more than once`;
    }

    serviceNames.add(normalizedServiceName);
  }

  return null;
}

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
  const [searchParams] = useSearchParams();
  const duplicateFrom = mode === 'add' ? searchParams.get('duplicateFrom') : null;
  const importPlatform = mode === 'add' ? searchParams.get('importPlatform') : null;
  const importProject = mode === 'add' ? searchParams.get('importProject') : null;
  const requestedImportKind = mode === 'add' ? searchParams.get('importKind') : null;
  const importKind =
    requestedImportKind === StackImportKind.ComposeProject || requestedImportKind === StackImportKind.SwarmStack
      ? requestedImportKind
      : undefined;
  const isComposeImport = !!importPlatform && !!importProject;
  const duplicateDraftLoadedRef = useRef<string | null>(null);
  const importDraftLoadedRef = useRef<string | null>(null);
  const validatedImportFingerprintRef = useRef<string | null>(null);
  const importConfirmationResolver = useRef<((confirmed: boolean) => void) | null>(null);
  const importSensitiveEnvironmentAsSecretsRef = useRef(true);
  const canImportSensitiveEnvironmentValuesRef = useRef(false);
  const [importConfirmationOpen, setImportConfirmationOpen] = useState(false);
  const [importValidation, setImportValidation] = useState<ComposeProjectImportValidation | null>(null);
  const [importSensitiveEnvironmentAsSecrets, setImportSensitiveEnvironmentAsSecrets] = useState(true);
  const [swarmPreflight, setSwarmPreflight] = useState<SwarmStackCompatibilityReport | null>(null);
  const [update, setUpdate] = useState<Partial<StackInput>>({});
  const queryClient = useQueryClient();
  const { hasCapability: hasLicenseCapability } = useLicenseEntitlements();
  const automatedOperationsEnabled = hasLicenseCapability(LicenseCapability.AutomatedOperations);
  const operationalGuardrailsEnabled = hasLicenseCapability(LicenseCapability.OperationalGuardrails);
  const updateSensitiveEnvironmentImport = useCallback((enabled: boolean) => {
    importSensitiveEnvironmentAsSecretsRef.current = enabled;
    setImportSensitiveEnvironmentAsSecrets(enabled);
  }, []);

  const { mutateAsync: createStack } = useMutate('createStack');
  const { mutateAsync: importComposeProject } = useMutate('importComposeProject');
  const { mutateAsync: validateComposeProjectImportDraft, isPending: isImportValidationPending } = useMutate(
    'validateComposeProjectImportDraft',
  );
  const { mutateAsync: updateStack } = useMutate('updateStack');
  const { mutateAsync: preflightSwarmStack, isPending: isSwarmPreflightPending } = useMutate('preflightSwarmStack');
  const { data: platformsData } = useRead('listPlatforms');
  const { data: stackCfg } = useRead('getStackConfig', { stackId: id });
  const { data: buildProjectsData, isFetching: buildProjectsLoading } = useRead('listBuildProjects');
  const { data: duplicateDraftData, isFetching: isDuplicateDraftLoading } = useRead(
    'getStackDuplicateDraft',
    { stackId: duplicateFrom ?? '' },
    { enabled: mode === 'add' && !!duplicateFrom },
  );
  const { data: importDraftData, isFetching: isImportDraftLoading } = useRead(
    'getComposeProjectImportDraft',
    {
      platformId: importPlatform ?? '',
      projectName: importProject ?? '',
      query: { importKind: importKind ?? undefined },
    },
    { enabled: mode === 'add' && isComposeImport },
  );
  const { data: stackViewData } = useRead('getStack', { stackId: id }, { enabled: mode === 'edit' && !!id });
  const { data: resourceBindingLookupData } = useRead('lookup', {
    query: {
      TargetResourceType: LookupResourceType.ResourceBinding,
      SourceResourceType: mode === 'edit' ? LookupResourceType.Stack : undefined,
      SourceResourceId: mode === 'edit' ? id : undefined,
    },
  });

  const resource: StackConfigView | undefined = stackCfg?.data;
  const duplicateDraft = duplicateDraftData?.data;
  const importDraft = importDraftData?.data;
  const isNativeSwarmImport = importDraft?.importKind === StackImportKind.SwarmStack;
  const importedPlatformId = isComposeImport
    ? (importDraft?.source.platformId ?? importPlatform ?? undefined)
    : undefined;
  const duplicateWarnings = duplicateDraft?.warnings ?? [];
  const importIssues = [...(importDraft?.issues ?? []), ...(importValidation?.issues ?? [])];
  const hasRuntimeImportBlocker = (importDraft?.issues ?? []).some(
    (issue) => issue.severity === AdoptionIssueSeverity.Blocker,
  );
  const formDraftKey = isComposeImport
    ? `stack:import:${importPlatform}:${importProject}:${importKind ?? 'auto'}`
    : duplicateFrom
      ? `stack:duplicate:${duplicateFrom}`
      : `stack:${id ?? 'new'}`;
  const scopedDraftKey = useFormDraftKey(formDraftKey);
  const stackView = stackViewData?.data;
  const original = resource ?? EMPTY_STACK_CONFIG;
  const formOriginal = useMemo(
    () =>
      normalizeDisabledWebhook(
        importedPlatformId ? ({ ...original, platformId: importedPlatformId } as StackConfigView) : original,
      ),
    [importedPlatformId, original],
  );
  const formUpdate = useMemo(() => normalizeDisabledWebhook(update), [update]);
  const currentStackSource = (update as Partial<CreateStackInput>).stackSource ?? original.stackSource;
  const currentPlatformId = importedPlatformId ?? (update as Partial<StackInput>).platformId ?? original.platformId;
  const stackPlatforms = useMemo(
    () =>
      (platformsData?.data.platforms ?? []).filter(
        (platform) => platform.type === PlatformType.Docker || platform.type === PlatformType.DockerSwarm,
      ),
    [platformsData?.data.platforms],
  );
  const lockedPlatformType =
    mode === 'edit'
      ? resource?.platformType
      : duplicateFrom
        ? stackPlatforms.find((platform) => platform.id === currentPlatformId)?.type
        : undefined;
  const selectablePlatforms = useMemo(
    () =>
      lockedPlatformType ? stackPlatforms.filter((platform) => platform.type === lockedPlatformType) : stackPlatforms,
    [lockedPlatformType, stackPlatforms],
  );
  const importedPlatformSelection = useMemo(
    () =>
      isComposeImport && importDraft?.source
        ? (stackPlatforms.find((platform) => platform.id === importedPlatformId) ??
          ({ id: importDraft.source.platformId, name: importDraft.source.platformName } as PlatformView))
        : undefined,
    [importDraft, importedPlatformId, isComposeImport, stackPlatforms],
  );
  const currentPlatformType =
    stackPlatforms.find((platform) => platform.id === currentPlatformId)?.type ?? lockedPlatformType;
  const isSwarmStack = currentPlatformType === PlatformType.DockerSwarm;
  const boundBuildServices = useMemo(
    () =>
      (
        (formUpdate.spec as StackSpec | undefined)?.buildImageBindings ??
        (formOriginal.spec as StackSpec | undefined)?.buildImageBindings ??
        []
      ).map((binding) => binding.serviceName),
    [formOriginal.spec, formUpdate.spec],
  );
  const canMoveDraftPlatform =
    mode === 'add' || (stackView?.status === StackReleaseStatus.Created && !stackView.source);
  const currentGitRepoId = (update as any)?.spec?.gitRepoId ?? (original.spec as any)?.gitRepoId ?? null;
  const currentGitBranch = (update as any)?.spec?.branch ?? (original.spec as any)?.branch ?? null;
  const currentPinnedCommit = (update as any)?.spec?.commitSha ?? (original.spec as any)?.commitSha ?? null;
  const { data: gitRepositoryData, isFetching: isGitRepositoryLoading } = useRead(
    'getGitRepository',
    { id: currentGitRepoId ?? '' },
    { enabled: currentStackSource === StackSource.Git && !!currentGitRepoId },
  );
  const currentGitRepository = gitRepositoryData?.data;
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
  const buildProjects = buildProjectsData?.data.projects ?? EMPTY_BUILD_PROJECTS;
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
    () =>
      [
        ...new Set([
          ...effectiveResourceBindings.map((entry) => entry.name),
          ...(importSensitiveEnvironmentAsSecrets ? (importValidation?.importableSensitiveEnvironmentNames ?? []) : []),
        ]),
      ].sort(),
    [
      effectiveResourceBindings,
      importSensitiveEnvironmentAsSecrets,
      importValidation?.importableSensitiveEnvironmentNames,
    ],
  );
  const licensedUpdateBehaviors = useMemo(
    () => ({
      ...update_behaviors,
      [StackUpdateBehavior.ServiceAutoDeploy]: {
        ...update_behaviors[StackUpdateBehavior.ServiceAutoDeploy],
        label: 'Auto Deploy Services',
        disabled: !operationalGuardrailsEnabled,
        requiredLicense: !operationalGuardrailsEnabled ? ('Team' as const) : undefined,
      },
      [StackUpdateBehavior.StackAutoDeploy]: {
        ...update_behaviors[StackUpdateBehavior.StackAutoDeploy],
        label: 'Auto Deploy Stack',
        disabled: !operationalGuardrailsEnabled,
        requiredLicense: !operationalGuardrailsEnabled ? ('Team' as const) : undefined,
      },
    }),
    [operationalGuardrailsEnabled],
  );
  const licensedDriftModes = useMemo(
    () => ({
      ...drift_modes,
      [StackDriftMode.DetectOnly]: {
        ...drift_modes[StackDriftMode.DetectOnly],
        label: 'Detect only',
        disabled: !operationalGuardrailsEnabled,
        requiredLicense: !operationalGuardrailsEnabled ? ('Team' as const) : undefined,
      },
      [StackDriftMode.AutoFix]: {
        ...drift_modes[StackDriftMode.AutoFix],
        label: 'Auto-fix safe drift',
        disabled: !operationalGuardrailsEnabled,
        requiredLicense: !operationalGuardrailsEnabled ? ('Team' as const) : undefined,
      },
    }),
    [operationalGuardrailsEnabled],
  );

  useEffect(() => {
    if (!duplicateFrom || !duplicateDraft?.draft || duplicateDraftLoadedRef.current === duplicateFrom) return;

    duplicateDraftLoadedRef.current = duplicateFrom;
    setUpdate(duplicateDraft.draft as Partial<StackInput>);
  }, [duplicateFrom, duplicateDraft?.draft]);

  useEffect(() => {
    if (!isComposeImport || !importDraft?.draft) return;

    const draftId = `${importPlatform}:${importProject}`;
    if (importDraftLoadedRef.current === draftId) return;

    importDraftLoadedRef.current = draftId;
    setUpdate(importDraft.draft as Partial<StackInput>);
  }, [isComposeImport, importDraft?.draft, importPlatform, importProject]);

  const setFormUpdate = useCallback(
    (value: Parameters<typeof setUpdate>[0]) => {
      setSwarmPreflight(null);
      setUpdate((previous) => {
        const changed = typeof value === 'function' ? value(previous) : value;
        const next = importedPlatformId ? { ...changed, platformId: importedPlatformId } : changed;
        if (!isComposeImport || !importProject) return next;

        const source = (next as Partial<CreateStackInput>).stackSource;
        if (!source) return next;

        const spec = (next as Partial<CreateStackInput>).spec as Partial<StackSpec> | undefined;
        return {
          ...next,
          stackSource: source,
          spec: {
            ...(spec ?? {}),
            $type: specTypeForSource(source),
            projectName: importProject,
            destroyBeforeDeploy: false,
          } as StackSpec,
          driftPolicy: DEFAULT_DRIFT_POLICY,
        };
      });
    },
    [importProject, isComposeImport, importedPlatformId],
  );

  const refreshData = useCallback(() => {
    if (scopedDraftKey) localStorage.removeItem(scopedDraftKey);
    queryClient.invalidateQueries({ queryKey: ['getStackConfig', { stackId: id }] });
    queryClient.invalidateQueries({ queryKey: ['getStack', { stackId: id }] });
    queryClient.invalidateQueries({ queryKey: ['getStackDrift', { stackId: id }] });
  }, [scopedDraftKey, id, queryClient]);

  useEffect(() => {
    if (!metadataChanged) return;
    refreshData();
  }, [metadataChanged, refreshData]);

  const { save: handleSave, isPending } = useSaveResource<StackInput, any>({
    mode,
    basePath: 'stacks',
    entityName: 'Stack',
    onCreate: (payload) => {
      const createPayload = payload as CreateStackInput;
      const normalizedPayload = {
        ...createPayload,
        spec: normalizeStackSpec(createPayload.spec, createPayload.stackSource),
      };

      if (isComposeImport && importDraft && importPlatform && importProject) {
        const previewFingerprint = validatedImportFingerprintRef.current;
        if (!previewFingerprint) {
          return Promise.reject(new Error('Validate the Compose source before importing it.'));
        }

        const importPayload: ImportRequest = {
          name: normalizedPayload.name,
          description: normalizedPayload.description,
          stackSource: normalizedPayload.stackSource,
          spec: normalizedPayload.spec,
          previewFingerprint,
          tagIds: normalizedPayload.tagIds,
          importKind: importDraft.importKind,
          importSensitiveEnvironmentAsSecrets:
            !isNativeSwarmImport &&
            canImportSensitiveEnvironmentValuesRef.current &&
            importSensitiveEnvironmentAsSecretsRef.current,
        };
        return importComposeProject({
          platformId: importPlatform,
          projectName: importProject,
          data: importPayload,
        });
      }

      return createStack({ data: normalizedPayload });
    },
    onUpdate: () =>
      updateStack({
        id,
        data: toPatchStackInput(update, original),
      }),
    onRefresh: refreshData,
  });

  const resolveImportConfirmation = useCallback((confirmed: boolean) => {
    const resolver = importConfirmationResolver.current;
    importConfirmationResolver.current = null;
    setImportConfirmationOpen(false);
    resolver?.(confirmed);
  }, []);
  const confirmComposeImport = useCallback(
    async (payload: StackInput) => {
      if (!importPlatform || !importProject) return false;

      const createPayload = payload as CreateStackInput;
      const response = await validateComposeProjectImportDraft({
        platformId: importPlatform,
        projectName: importProject,
        data: {
          name: createPayload.name,
          stackSource: createPayload.stackSource,
          spec: createPayload.spec,
          importKind: importDraft?.importKind,
        },
      });
      const validation = response.data;
      setImportValidation(validation);
      canImportSensitiveEnvironmentValuesRef.current = validation.canImportSensitiveEnvironmentValues;
      validatedImportFingerprintRef.current = null;
      if (validation.issues.some((issue) => issue.severity === AdoptionIssueSeverity.Blocker)) {
        toast.error('The selected source does not match the running Compose project.');
        return false;
      }

      validatedImportFingerprintRef.current = validation.previewFingerprint;
      return new Promise<boolean>((resolve) => {
        importConfirmationResolver.current = resolve;
        setImportConfirmationOpen(true);
      });
    },
    [importDraft?.importKind, importPlatform, importProject, validateComposeProjectImportDraft],
  );

  const confirmSave = useCallback(
    async (payload: StackInput) => {
      if (isComposeImport) return confirmComposeImport(payload);

      const platformType =
        stackPlatforms.find((platform) => platform.id === payload.platformId)?.type ?? lockedPlatformType;
      if (platformType !== PlatformType.DockerSwarm) {
        setSwarmPreflight(null);
        return true;
      }

      if (swarmPreflight?.isCompatible && swarmPreflight.issues.length > 0) return true;

      const response = await preflightSwarmStack({
        data: {
          name: payload.name ?? original.name ?? '',
          platformId: payload.platformId ?? original.platformId ?? '',
          stackSource: payload.stackSource ?? original.stackSource,
          spec: normalizeStackSpec(payload.spec, payload.stackSource ?? original.stackSource),
          driftPolicy: payload.driftPolicy,
        },
      });
      setSwarmPreflight(response.data);
      if (!response.data.isCompatible) {
        toast.error('Resolve the Swarm compatibility errors before saving.');
      } else if (response.data.issues.length > 0) {
        toast.warning('Review the Swarm portability warnings, then save again to continue.');
        return false;
      }

      return response.data.isCompatible;
    },
    [
      confirmComposeImport,
      isComposeImport,
      lockedPlatformType,
      original.name,
      original.stackSource,
      preflightSwarmStack,
      stackPlatforms,
      swarmPreflight,
    ],
  );

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
                    defineField<StackInput, 'name'>({
                      key: 'name',
                      label: 'Name',
                      required: true,
                      description: 'Internal identifier for this workload.',
                      validate: (v) => (!v ? 'Name is required' : null),
                      render: (val, set) => (
                        <FieldInput value={val} onChange={(v) => set({ name: v })} placeholder="stack-name" />
                      ),
                    }),
                    defineField<StackInput, 'description'>({
                      key: 'description',
                      label: 'Description',
                      required: false,
                      description: 'Optional description of this workload.',
                      render: (val, set) => <FieldTextArea value={val} onChange={(v) => set({ description: v })} />,
                    }),
                    defineField<StackInput, 'tagIds'>({
                      key: 'tagIds',
                      label: 'Tags',
                      required: false,
                      description: 'Optional tags for filtering and grouping this stack.',
                      render: (val, set) => (
                        <ResourceTagSelector value={val} disabled={disabled} onChange={(tagIds) => set({ tagIds })} />
                      ),
                    }),
                  ],
                }),
              ]
            : []),
          defineField<StackInput, 'platformId'>({
            key: 'platformId',
            label: 'Platform',
            required: true,
            disabled: !canMoveDraftPlatform || isComposeImport,
            description: !canMoveDraftPlatform
              ? 'Applied Stacks cannot move platforms in this release.'
              : lockedPlatformType
                ? `Select another ${lockedPlatformType === PlatformType.DockerSwarm ? 'Docker Swarm' : 'Docker Standalone'} platform.`
                : 'Select a Docker Standalone or Docker Swarm platform. The selected type is locked after creation.',
            render: (value, set) => {
              return (
                <ResourceSelectorField<PlatformView>
                  sourceType={LookupResourceType.Stack}
                  targetType={LookupResourceType.Platform}
                  selected={importedPlatformSelection ?? value}
                  items={selectablePlatforms}
                  renderItem={(platform) => (
                    <span className="flex min-w-0 items-center justify-between gap-3">
                      <span className="truncate">{platform.name}</span>
                      <span className="shrink-0 text-xs text-muted-foreground">
                        {platform.type === PlatformType.DockerSwarm ? 'Swarm' : 'Standalone'}
                      </span>
                    </span>
                  )}
                  onSelect={(platform) =>
                    set((previous) => ({
                      platformId: platform?.id,
                      ...(platform?.type === PlatformType.DockerSwarm
                        ? {
                            driftPolicy: DEFAULT_DRIFT_POLICY,
                            spec: {
                              ...previous.spec,
                              destroyBeforeDeploy: false,
                              preDeploy: null,
                              postDeploy: null,
                            } as StackSpec,
                          }
                        : {}),
                    }))
                  }
                  placeholder="Select Platform"
                  allowClear={false}
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
                  onChange={(v: StackSource) =>
                    set((previous) => ({
                      stackSource: v,
                      spec: normalizeStackSpec((previous.spec ?? {}) as StackSpec, v),
                    }))
                  }
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
                    defineField<StackInput, 'spec.gitRepoId'>({
                      key: 'spec.gitRepoId',
                      label: 'Repository',
                      required: true,
                      description: 'Repository that contains the Compose project for this stack.',
                      validate: (v) => (!v ? 'Repository is required' : null),
                      render: (value, set) => (
                        <div className="flex w-full max-w-100 items-center gap-2">
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
                            className="max-w-none flex-1"
                          />
                          <GitRepositoryBrowseAction
                            resource={currentGitRepository}
                            title="Browse"
                            branch={currentGitBranch}
                            commitSha={currentPinnedCommit}
                            loading={isGitRepositoryLoading}
                            className="flex-none"
                            disabledReason={
                              currentGitRepoId
                                ? 'The selected repository is unavailable.'
                                : 'Select a repository to browse.'
                            }
                          />
                        </div>
                      ),
                    }),
                    defineField<StackInput, 'spec.branch'>({
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
                    defineField<StackInput, 'spec.commitSha'>({
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
                    defineField<StackInput, 'spec.composePaths'>({
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
                    defineField<StackInput, 'spec.composeEnvFilesFromRepo'>({
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
                      defineField<StackInput, 'spec.composeFile'>({
                        key: 'spec.composeFile',
                        label: 'Compose File',
                        required: true,
                        description: 'Define your docker compose YAML content.',
                        validate: (v) => (!v ? 'Compose file is required' : null),
                        render: (value, set) => (
                          <MonacoEditor
                            language="yaml"
                            filename={isSwarmStack ? 'swarm-compose.yaml' : 'compose.yaml'}
                            value={value ?? DEFAULT_STACK_FILE_CONTENTS}
                            diagnostics={[
                              ...getComposeVariableDiagnostics(
                                value ?? DEFAULT_STACK_FILE_CONTENTS,
                                effectiveConfigurationNames,
                              ),
                              ...(isSwarmStack
                                ? getSwarmComposeDiagnostics(value ?? DEFAULT_STACK_FILE_CONTENTS, boundBuildServices)
                                : []),
                            ]}
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
                          collection={licensedUpdateBehaviors}
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
                defineGroupField<StackInput>({
                  id: 'build_image_bindings',
                  label: 'Build Images',
                  title: 'Build Images',
                  description:
                    'Map Compose services to Citadel builds. When a mapped build succeeds, Citadel updates the service image reference and can redeploy that service.',
                  items: [
                    defineField<StackInput, 'spec.buildImageBindings'>({
                      key: 'spec.buildImageBindings',
                      label: 'Service Bindings',
                      description: 'Use the exact Compose service name, then select the build that produces its image.',
                      validate: validateStackBuildImageBindings,
                      render: (value, set) => (
                        <StackBuildImageBindingsField
                          value={value}
                          projects={buildProjects}
                          isLoading={buildProjectsLoading}
                          disabled={disabled}
                          redeployEnabled={automatedOperationsEnabled}
                          onChange={(buildImageBindings) =>
                            set((prev) => ({
                              spec: {
                                ...prev.spec!,
                                buildImageBindings,
                              },
                            }))
                          }
                        />
                      ),
                    }),
                  ],
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
                  disabled: isComposeImport,
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
                ...(!isSwarmStack
                  ? [
                      defineGroupField<StackInput>({
                        id: 'drift_policy',
                        label: 'Drift Management',
                        title: 'Drift Management',
                        description: isComposeImport
                          ? 'Disabled during import. Edit the stack after importing it to configure drift management.'
                          : 'Detect runtime differences between the compose file and the containers currently running on the platform.',
                        items: [
                          defineField<StackInput, 'driftPolicy.mode'>({
                            key: 'driftPolicy.mode',
                            label: 'Mode',
                            description: 'Choose how this stack handles drift checks.',
                            disabled: isComposeImport,
                            render: (value, set) => (
                              <ItemSelector
                                collection={licensedDriftModes}
                                value={value ?? StackDriftMode.Disabled}
                                disabled={disabled}
                                onChange={(mode: StackDriftMode) =>
                                  set((prev) => patchDriftPolicy(prev, getDriftModePreset(mode)))
                                }
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
                                      onChange={(alertOnDrift) =>
                                        set((prev) => patchDriftPolicy(prev, { alertOnDrift }))
                                      }
                                    />
                                  ),
                                }),
                                defineField<StackInput, 'driftPolicy.markDegraded'>({
                                  key: 'driftPolicy.markDegraded',
                                  label: 'Mark Degraded',
                                  description:
                                    'Mark the stack degraded and record an activity event when drift is detected.',
                                  render: (value, set) => (
                                    <FieldSwitch
                                      id="stack-drift-mark-degraded"
                                      checked={value ?? currentDriftPolicy.markDegraded}
                                      disabled={disabled}
                                      onChange={(markDegraded) =>
                                        set((prev) => patchDriftPolicy(prev, { markDegraded }))
                                      }
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
                          defineField<StackInput, 'spec.preDeploy.path'>({
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
                          defineField<StackInput, 'spec.preDeploy.commands'>({
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
                          defineField<StackInput, 'spec.postDeploy.path'>({
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
                          defineField<StackInput, 'spec.postDeploy.commands'>({
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

                      defineField<StackInput, 'spec.destroyBeforeDeploy'>({
                        key: 'spec.destroyBeforeDeploy',
                        label: 'Destroy',
                        description: `Ensure 'docker compose down' is run before redeploying the Stack.`,
                        required: false,
                        disabled: isComposeImport,
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
                    ]
                  : []),
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
      currentGitRepository,
      isGitRepositoryLoading,
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
      buildProjects,
      buildProjectsLoading,
      automatedOperationsEnabled,
      licensedUpdateBehaviors,
      licensedDriftModes,
      selectedBranchRef?.resolvedCommitSha,
      stackView?.source?.resolvedCommitSha,
      effectiveConfigurationNames,
      boundBuildServices,
      isComposeImport,
      isSwarmStack,
      canMoveDraftPlatform,
      lockedPlatformType,
      selectablePlatforms,
      importedPlatformSelection,
    ],
  );

  return (
    <div className="flex flex-col gap-3">
      {duplicateFrom && (
        <AlertMessage type="info" title={isDuplicateDraftLoading ? 'Loading duplicate draft' : 'Duplicate draft'}>
          No stack has been created yet. Review the copied configuration, then save it to create the stack.
        </AlertMessage>
      )}
      {duplicateWarnings.map((warning) => (
        <AlertMessage key={`${warning.code}:${warning.fieldPath ?? ''}`} type="warning" title={warning.code}>
          {warning.message}
        </AlertMessage>
      ))}
      {isComposeImport && (
        <AlertMessage
          type="info"
          title={
            isImportDraftLoading
              ? importKind === StackImportKind.SwarmStack
                ? 'Inspecting Docker Stack'
                : 'Inspecting Compose project'
              : isNativeSwarmImport
                ? 'Import Docker Stack'
                : 'Import Compose project'
          }>
          Select the Web Editor or Git source that defines this {isNativeSwarmImport ? 'Docker Stack' : 'project'}.
          Citadel will compare it with the running services, then associate the existing runtime without applying or
          restarting it.{' '}
          {!isNativeSwarmImport && isSwarmStack && 'The first Apply will convert it to a native Docker Swarm Stack.'}
        </AlertMessage>
      )}
      {isComposeImport && importDraft && !isNativeSwarmImport && (
        <div className="flex items-start justify-between gap-4 rounded-md border px-3 py-3">
          <div className="flex min-w-0 gap-3">
            <KeyRound className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
            <div className="min-w-0">
              <div className="text-sm font-medium">Import detected values as Citadel secrets</div>
              <div className="mt-1 text-xs text-muted-foreground">
                {importValidation?.canImportSensitiveEnvironmentValues
                  ? `Encrypt and bind ${importValidation.importableSensitiveEnvironmentNames.join(', ')} to this Stack.`
                  : 'Encrypt and bind matching sensitive values referenced by the reviewed Compose source.'}{' '}
                The values never leave the server.
              </div>
            </div>
          </div>
          <Switch
            checked={importSensitiveEnvironmentAsSecrets}
            onCheckedChange={updateSensitiveEnvironmentImport}
            aria-label="Import detected values as Citadel secrets"
          />
        </div>
      )}
      {importIssues.map((issue) => (
        <AlertMessage
          key={`${issue.code}:${issue.fieldPath ?? ''}`}
          type={issue.severity === AdoptionIssueSeverity.Blocker ? 'error' : 'warning'}
          title={issue.severity === AdoptionIssueSeverity.Blocker ? 'Import blocked' : 'Review required'}>
          {issue.message}
        </AlertMessage>
      ))}
      {importValidation && (
        <AlertMessage type="info" title="Source comparison">
          {
            importValidation.services.filter(
              (service) => Number(service.runtimeContainerCount) > 0 && service.definedInSource,
            ).length
          }{' '}
          of {importDraft?.source.services.length ?? 0} running services match the selected source.
        </AlertMessage>
      )}
      {isSwarmStack && !isComposeImport && (
        <AlertMessage type="info" title="Docker Swarm Stack">
          Saving validates this Stack for Docker Swarm. Select Apply after saving to deploy it to the cluster.
        </AlertMessage>
      )}
      {swarmPreflight?.issues.map((issue) => (
        <AlertMessage
          key={`${issue.code}:${issue.fieldPath ?? ''}`}
          type={issue.severity === SwarmStackCompatibilitySeverity.Error ? 'error' : 'warning'}
          title={
            issue.severity === SwarmStackCompatibilitySeverity.Error ? 'Compatibility error' : 'Portability warning'
          }>
          {issue.message}
        </AlertMessage>
      ))}
      {swarmPreflight?.isCompatible && swarmPreflight.issues.length === 0 && (
        <AlertMessage type="success" title="Swarm compatible">
          No compatibility issues were found.
        </AlertMessage>
      )}
      <FormShell
        mode={mode}
        schema={schema}
        original={formOriginal}
        update={formUpdate}
        setUpdate={setFormUpdate}
        onSave={handleSave}
        pending={isPending || isSwarmPreflightPending}
        disabled={disabled}
        saveDisabled={
          isComposeImport &&
          (isImportDraftLoading || isImportValidationPending || !importDraft || hasRuntimeImportBlocker)
        }
        saveLabel={isComposeImport ? (isNativeSwarmImport ? 'Import Stack' : 'Import Project') : 'Save'}
        confirmSave={isComposeImport || isSwarmStack ? confirmSave : undefined}
        draftKey={formDraftKey}
        draftVersion={1}
      />
      <ActionWithDialog
        renderTrigger={false}
        open={importConfirmationOpen}
        onOpenChange={(open) => !open && resolveImportConfirmation(false)}
        name={importProject ?? ''}
        title="Import"
        icon={<FolderInput className="h-4 w-4" />}
        disabled={!importProject}
        onClick={() => resolveImportConfirmation(true)}
        description={
          <>
            Citadel will start managing the existing{' '}
            {isNativeSwarmImport ? 'Docker Stack Services' : 'project containers'} as one Stack. Docker will not be
            changed now. Future Apply operations will use the reviewed{' '}
            {currentStackSource === StackSource.Git ? 'Git' : 'Web Editor'} source.
            {!isNativeSwarmImport && isSwarmStack
              ? ' On first Apply, Citadel will stop the Compose project without deleting its volumes and deploy it through Docker Swarm.'
              : ''}
          </>
        }
      />
    </div>
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
