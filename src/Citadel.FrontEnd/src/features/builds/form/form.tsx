import {
  BuildArgSpec,
  BuildWebhookConfig,
  GitRepositoryBranchView,
  GitRepositoryRefView,
  BuildProjectInput,
  BuildProjectView,
  BuildSecretSpec,
  LookupResourceType,
  PlatformConnectorType,
  UpdateBuildProjectInput,
} from '@/api/generated/api.types';
import { ResourceSelectorField } from '@/components/custom/common';
import { WebhookConfigField } from '@/components/custom/webhook-config-field';
import {
  FieldInput,
  FieldSelect,
  FieldSwitch,
  FieldTextArea,
  FormShell,
  defineField,
  defineGroupField,
  defineRowField,
  defineSection,
} from '@/components/custom/form-builder';
import { ResourceTagSelector } from '@/features/tags/components';
import { Constants } from '@/lib/constants';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { MonacoEditor, type MonacoDiagnostic } from '@/lib/monaco';
import { useQueryClient } from '@tanstack/react-query';
import { GitBranch } from 'lucide-react';
import { type ReactNode, useMemo, useState } from 'react';
import { useParams } from 'react-router';

type BuildInput = BuildProjectInput | UpdateBuildProjectInput;

const defaultBuild: BuildProjectInput = {
  name: '',
  description: null,
  enabled: true,
  gitRepositoryId: '',
  branch: 'main',
  contextPath: '.',
  dockerfilePath: 'Dockerfile',
  target: null,
  buildArgs: [],
  buildSecrets: [],
  platformId: '',
  registryId: '',
  imageRepository: '',
  tagTemplates: ['{branch}-{shortSha}'],
  webhook: { enabled: false },
  timeoutSeconds: 1800,
  retentionRunCount: 20,
  tagIds: [],
};

const shortSha = (value?: string | null) => (value ? value.slice(0, 12) : '-');
const agentContextLimitLabel = '16 MB';

type GitBranchOption = {
  branch: string;
  commitSha?: string | null;
  lastError?: string | null;
};

const GitBranchField = ({
  repoId,
  value,
  branches,
  isFetching,
  discoveryFailed,
  onChange,
}: {
  repoId?: string | null;
  value?: string | null;
  branches: GitBranchOption[];
  isFetching?: boolean;
  discoveryFailed?: boolean;
  onChange: (branch: string) => void;
}) => {
  const options = useMemo(() => {
    return branches.map((option) => ({
      value: option.branch,
      label: (
        <span className="flex min-w-0 items-center gap-2">
          <GitBranch className="size-3.5 text-muted-foreground" />
          <span className="truncate">{option.branch}</span>
          {option?.commitSha ? (
            <span className="font-mono text-xs text-muted-foreground">{shortSha(option.commitSha)}</span>
          ) : null}
        </span>
      ),
    }));
  }, [branches]);

  if (!repoId || options.length === 0) {
    return (
      <div className="flex flex-col gap-2">
        <FieldInput value={value ?? ''} onChange={onChange} placeholder="main" />
        <p className="text-xs text-muted-foreground">
          {isFetching
            ? 'Discovering branches from the selected repository.'
            : discoveryFailed
              ? 'Could not discover remote branches. Check repository access, or type a branch manually.'
              : 'Select a repository to discover branches. You can still type a branch manually.'}
        </p>
      </div>
    );
  }

  const selectedBranch = branches.find((branch) => branch.branch === value);
  const hasSelectedBranch = Boolean(selectedBranch);

  return (
    <div className="flex flex-col gap-2">
      <FieldSelect
        value={hasSelectedBranch ? (value ?? undefined) : undefined}
        onChange={onChange}
        options={options}
        placeholder="Select branch"
      />
      {value && !hasSelectedBranch ? (
        <p className="text-xs text-destructive">Branch &quot;{value}&quot; was not found in the selected repository.</p>
      ) : null}
      {discoveryFailed ? (
        <p className="text-xs text-muted-foreground">Could not refresh remote branches; showing synced refs.</p>
      ) : null}
      {selectedBranch?.lastError ? <p className="text-xs text-destructive">{selectedBranch.lastError}</p> : null}
    </div>
  );
};

const BuildPlatformField = ({
  value,
  options,
  hasLoaded,
  isLoading,
  selectedConnectorType,
  disabled,
  onChange,
}: {
  value?: string | null;
  options: Array<{ value: string; label: ReactNode }>;
  hasLoaded: boolean;
  isLoading?: boolean;
  selectedConnectorType?: PlatformConnectorType;
  disabled?: boolean;
  onChange: (platformId: string) => void;
}) => {
  const isSupportedSelection = options.some((option) => option.value === value);
  const hasUnsupportedSelection = hasLoaded && Boolean(value) && !isSupportedSelection;
  const message = hasUnsupportedSelection
    ? `This build uses a ${formatConnectorType(selectedConnectorType)} platform that is no longer available. Select another platform before saving.`
    : options.length === 0 && hasLoaded
      ? 'No Docker platforms are available. Add a platform before creating a build.'
      : isAgentConnector(selectedConnectorType)
        ? `Agent builds package the selected context and transfer it to the agent. Keep it below ${agentContextLimitLabel} after .dockerignore filtering.`
        : 'Select the Docker platform that runs this build. Local, agent, and edge-agent connectors are supported.';

  return (
    <div className="flex flex-col gap-2">
      <FieldSelect
        value={isSupportedSelection ? (value ?? undefined) : undefined}
        onChange={onChange}
        options={options}
        disabled={disabled || isLoading || options.length === 0}
        placeholder={isLoading ? 'Loading platforms...' : 'Select platform'}
      />
      <p className={hasUnsupportedSelection ? 'text-xs text-destructive' : 'text-xs text-muted-foreground'}>
        {message}
      </p>
    </div>
  );
};

function formatConnectorType(connectorType?: PlatformConnectorType) {
  switch (connectorType) {
    case PlatformConnectorType.Agent:
      return 'agent';
    case PlatformConnectorType.EdgeAgent:
      return 'edge-agent';
    case PlatformConnectorType.Local:
      return 'local';
    default:
      return 'unsupported';
  }
}

function isAgentConnector(connectorType?: PlatformConnectorType) {
  return connectorType === PlatformConnectorType.Agent || connectorType === PlatformConnectorType.EdgeAgent;
}

const JsonArrayEditor = <T,>({
  value,
  filename,
  helperText,
  onChange,
}: {
  value: T[];
  filename: string;
  helperText: string;
  onChange: (value: T[]) => void;
}) => {
  const [raw, setRaw] = useState(() => formatJson(value));
  const [diagnostics, setDiagnostics] = useState<MonacoDiagnostic[]>([]);

  return (
    <div className="flex flex-col gap-2">
      <MonacoEditor
        language="json"
        filename={filename}
        value={raw}
        minHeight={180}
        diagnostics={diagnostics}
        onValueChange={(next) => {
          setRaw(next);
          const result = parseJsonArrayWithDiagnostics<T>(next);
          setDiagnostics(result.diagnostics);
          if (result.value) onChange(result.value);
        }}
      />
      <p className="text-xs text-muted-foreground">{helperText}</p>
    </div>
  );
};

export const BuildForm = ({
  mode,
  resource,
  disabled,
  metadataChanged,
}: {
  mode: 'add' | 'edit';
  resource?: BuildProjectView;
  disabled?: boolean;
  metadataChanged?: boolean;
}) => {
  const id = useParams().id;
  const queryClient = useQueryClient();
  const [update, setUpdate] = useState<Partial<BuildInput>>({});
  const createBuild = useMutate('createBuildProject');
  const updateBuild = useMutate('updateBuildProject');
  const currentGitRepositoryId = update.gitRepositoryId ?? resource?.gitRepositoryId ?? defaultBuild.gitRepositoryId;
  const { data: gitRefsData, isFetching: isFetchingGitRefs } = useRead(
    'getGitRepositoryRefs',
    { id: currentGitRepositoryId ?? '' },
    { enabled: !!currentGitRepositoryId },
  );
  const {
    data: gitBranchesData,
    isFetching: isFetchingGitBranches,
    isError: isGitBranchesError,
  } = useRead(
    'discoverGitRepositoryBranches',
    { id: currentGitRepositoryId ?? '' },
    { enabled: !!currentGitRepositoryId },
  );
  const { data: platformsData, isLoading: isLoadingPlatforms } = useRead('listPlatforms');
  const gitRefs = useMemo(() => gitRefsData?.data.refs ?? [], [gitRefsData?.data.refs]);
  const gitBranches = useMemo(
    () => mapBranchOptions(gitBranchesData?.data.branches ?? [], gitRefs),
    [gitBranchesData?.data.branches, gitRefs],
  );
  const platforms = useMemo(() => platformsData?.data.platforms ?? [], [platformsData?.data.platforms]);
  const currentPlatformId = update.platformId ?? resource?.platformId ?? defaultBuild.platformId;
  const selectedPlatform = useMemo(
    () => platforms.find((platform) => platform.id === currentPlatformId),
    [currentPlatformId, platforms],
  );
  const platformOptions = useMemo(
    () =>
      platforms.map((platform) => ({
        value: platform.id,
        label: (
          <span className="flex min-w-0 items-center gap-2">
            <span className="truncate">{platform.name}</span>
            <span className="text-xs text-muted-foreground">{formatConnectorType(platform.connectorType)}</span>
          </span>
        ),
      })),
    [platforms],
  );

  const { save: handleSave, isPending } = useSaveResource<BuildInput, any>({
    mode,
    basePath: 'builds',
    entityName: 'Build',
    onCreate: (payload) => createBuild.mutateAsync({ data: normalizePayload(payload, mode) as BuildProjectInput }),
    onUpdate: (payload) =>
      updateBuild.mutateAsync({ id: id!, data: normalizePayload(payload, mode) as UpdateBuildProjectInput }),
    onRefresh: () => {
      localStorage.removeItem(`Build:${id ?? 'new'}`);
      queryClient.invalidateQueries({ queryKey: ['getBuildProject', { id }] });
      queryClient.invalidateQueries({ queryKey: ['listBuildProjects'] });
    },
    extractName: (payload, response) =>
      response?.data?.name ?? ('name' in payload ? payload.name : undefined) ?? resource?.name ?? 'Build',
  });

  const original = useMemo<BuildInput>(() => {
    if (!resource) return defaultBuild;

    return {
      description: resource.description,
      enabled: resource.enabled,
      gitRepositoryId: resource.gitRepositoryId,
      branch: resource.branch,
      contextPath: resource.contextPath,
      dockerfilePath: resource.dockerfilePath,
      target: resource.target,
      buildArgs: resource.buildArgs,
      buildSecrets: resource.buildSecrets,
      platformId: resource.platformId,
      registryId: resource.registryId,
      imageRepository: resource.imageRepository,
      tagTemplates: resource.tagTemplates,
      webhook: resource.webhook ?? { enabled: false },
      timeoutSeconds: resource.timeoutSeconds,
      retentionRunCount: resource.retentionRunCount,
    };
  }, [resource]);

  const schema = useMemo(
    () => ({
      Source: defineSection<BuildInput>({
        title: 'Source',
        items: [
          ...(mode === 'add'
            ? [
                defineGroupField<BuildInput>({
                  id: 'details',
                  label: 'Details',
                  items: [
                    defineField({
                      key: 'name',
                      label: 'Name',
                      description: 'Internal identifier for this workload.',
                      required: true,
                      validate: (v) =>
                        !new RegExp(Constants.validNameIdentifier).test(v ?? '') ? 'Invalid name format' : null,
                      render: (value, set) => (
                        <FieldInput value={value ?? ''} onChange={(v) => set({ name: v })} placeholder="api-image" />
                      ),
                    }),
                    defineField({
                      key: 'description',
                      label: 'Description',
                      description: 'Optional description of this workload.',
                      render: (value, set) => (
                        <FieldTextArea value={value ?? ''} onChange={(v) => set({ description: v })} />
                      ),
                    }),
                    defineField({
                      key: 'tagIds',
                      label: 'Tags',
                      description: 'Optional tags for filtering and grouping build projects.',
                      render: (value, set) => (
                        <ResourceTagSelector value={value} onChange={(tagIds) => set({ tagIds })} />
                      ),
                    }),
                  ],
                }),
              ]
            : []),
          defineGroupField<BuildInput>({
            id: 'source',
            label: 'Git Source',
            items: [
              defineField({
                key: 'gitRepositoryId',
                label: 'Repository',
                description: 'Git repository that contains the Docker build context and Dockerfile.',
                required: true,
                render: (value, set) => (
                  <ResourceSelectorField
                    targetType={LookupResourceType.GitRepository}
                    selected={value}
                    onSelect={(item) => set({ gitRepositoryId: item?.id ?? '' })}
                    placeholder="Select repository"
                  />
                ),
              }),
              defineField({
                key: 'branch',
                label: 'Branch',
                description:
                  'Branch Citadel checks out before building. Branches are discovered from the selected repository.',
                required: true,
                validate: (value) =>
                  gitBranches.length > 0 && !gitBranches.some((branch) => branch.branch === value)
                    ? 'Select a valid repository branch.'
                    : null,
                render: (value, set) => (
                  <GitBranchField
                    repoId={currentGitRepositoryId}
                    value={value ?? ''}
                    branches={gitBranches}
                    isFetching={isFetchingGitBranches || isFetchingGitRefs}
                    discoveryFailed={isGitBranchesError}
                    onChange={(branch) => set({ branch })}
                  />
                ),
              }),
              defineField({
                key: 'contextPath',
                label: 'Context',
                description: 'Directory, relative to the repository root, sent to Docker as the build context.',
                required: true,
                validate: validateRepoRelativePath,
                render: (value, set) => (
                  <div className="flex flex-col gap-2">
                    <FieldInput value={value ?? ''} onChange={(v) => set({ contextPath: v })} placeholder="." />
                    <p className="text-xs text-muted-foreground">
                      {isAgentConnector(selectedPlatform?.connectorType)
                        ? `Agent and edge-agent builds upload this directory to the runner. Use .dockerignore to keep the transferred context under ${agentContextLimitLabel}.`
                        : 'Docker receives this directory as the build context. Use .dockerignore to keep builds fast.'}
                    </p>
                  </div>
                ),
              }),
              defineField({
                key: 'dockerfilePath',
                label: 'Dockerfile',
                description: 'Dockerfile path, relative to the repository root.',
                required: true,
                validate: validateRepoRelativePath,
                render: (value, set) => (
                  <FieldInput
                    value={value ?? ''}
                    onChange={(v) => set({ dockerfilePath: v })}
                    placeholder="Dockerfile"
                  />
                ),
              }),
              defineField({
                key: 'target',
                label: 'Target stage',
                description:
                  'Optional multi-stage Dockerfile target to build. Leave blank to build the default final stage.',
                render: (value, set) => <FieldInput value={value ?? ''} onChange={(v) => set({ target: v || null })} />,
              }),
            ],
          }),
        ],
      }),
      Output: defineSection<BuildInput>({
        title: 'Output',
        items: [
          defineGroupField<BuildInput>({
            id: 'runner',
            label: 'Runner and Registry',
            items: [
              defineField({
                key: 'platformId',
                label: 'Platform',
                description:
                  'Docker platform that runs the build. Local, agent, and edge-agent connectors are supported.',
                required: true,
                validate: (value) =>
                  platformsData && value && !platforms.some((platform) => platform.id === value)
                    ? 'Select an available Docker platform.'
                    : null,
                render: (value, set) => (
                  <BuildPlatformField
                    value={value}
                    options={platformOptions}
                    hasLoaded={Boolean(platformsData)}
                    isLoading={isLoadingPlatforms}
                    selectedConnectorType={platforms.find((platform) => platform.id === value)?.connectorType}
                    disabled={disabled}
                    onChange={(platformId) => set({ platformId })}
                  />
                ),
              }),
              defineField({
                key: 'registryId',
                label: 'Registry',
                description: 'Registry Citadel pushes the generated image tags to.',
                required: true,
                render: (value, set) => (
                  <ResourceSelectorField
                    targetType={LookupResourceType.Registry}
                    selected={value}
                    onSelect={(item) => set({ registryId: item?.id ?? '' })}
                    placeholder="Select registry"
                  />
                ),
              }),
              defineField({
                key: 'imageRepository',
                label: 'Image repository',
                description: 'Repository name under the selected registry, for example team/api.',
                required: true,
                render: (value, set) => (
                  <FieldInput
                    value={value ?? ''}
                    onChange={(v) => set({ imageRepository: v })}
                    placeholder="team/api"
                  />
                ),
              }),
              defineField({
                key: 'tagTemplates',
                label: 'Tags',
                required: true,
                description: 'Comma-separated tag templates. Supported tokens include {branch}, {shortSha}, and {sha}.',
                render: (value, set) => (
                  <FieldInput
                    value={(value ?? []).join(', ')}
                    onChange={(v) =>
                      set({
                        tagTemplates: String(v)
                          .split(',')
                          .map((item) => item.trim())
                          .filter(Boolean),
                      })
                    }
                    placeholder="{branch}-{shortSha}, latest"
                  />
                ),
              }),
            ],
          }),
          defineGroupField<BuildInput>({
            id: 'webhook',
            label: 'Webhook',
            description: 'Allow a Git provider webhook to queue this build when matching source files change.',
            items: [
              defineField<BuildInput, 'webhook'>({
                key: 'webhook',
                label: 'Enabled',
                render: (value, set) => (
                  <WebhookConfigField
                    resourceType="build"
                    resourceId={id}
                    execution="run"
                    value={value ?? { enabled: false }}
                    defaultBranch={(update.branch ?? original.branch) as string | null | undefined}
                    disabled={disabled}
                    onChange={(webhook) => set({ webhook: webhook as BuildWebhookConfig })}
                  />
                ),
              }),
            ],
          }),
        ],
      }),
      Build: defineSection<BuildInput>({
        title: 'Build',
        items: [
          defineGroupField<BuildInput>({
            id: 'options',
            label: 'Options',
            items: [
              defineField({
                key: 'enabled',
                label: 'Enabled',
                description: 'Disabled builds cannot be queued manually or by future webhook automation.',
                render: (value, set) => (
                  <FieldSwitch id="build-enabled" checked={value !== false} onChange={(enabled) => set({ enabled })} />
                ),
              }),
              defineRowField<BuildInput>({
                id: 'limits',
                fields: [
                  defineField({
                    key: 'timeoutSeconds',
                    label: 'Timeout seconds',
                    description: 'Maximum time Docker may spend building and pushing before the run is timed out.',
                    required: true,
                    render: (value, set) => (
                      <FieldInput type="number" value={value ?? 1800} onChange={(v) => set({ timeoutSeconds: v })} />
                    ),
                  }),
                  defineField({
                    key: 'retentionRunCount',
                    label: 'Run retention',
                    description: 'Number of recent run records to keep for this build project.',
                    required: true,
                    render: (value, set) => (
                      <FieldInput type="number" value={value ?? 20} onChange={(v) => set({ retentionRunCount: v })} />
                    ),
                  }),
                ],
              }),
              defineField({
                key: 'buildArgs',
                label: 'Build arguments',
                description: 'JSON array of build args, for example [{"name":"NODE_ENV","value":"production"}].',
                validate: validateJsonArray,
                render: (value, set) => (
                  <JsonArrayEditor<BuildArgSpec>
                    value={value ?? []}
                    filename="build-arguments.json"
                    helperText="Each item becomes a Docker build arg. Do not put secrets here; Docker can expose build args in image metadata."
                    onChange={(buildArgs) => set({ buildArgs })}
                  />
                ),
              }),
              defineField({
                key: 'buildSecrets',
                label: 'Build secrets',
                description: 'JSON array of secret mounts, for example [{"id":"npm_token","secretId":"..."}].',
                validate: validateJsonArray,
                render: (value, set) => (
                  <JsonArrayEditor<BuildSecretSpec>
                    value={value ?? []}
                    filename="build-secrets.json"
                    helperText="Reserved for BuildKit-native builders. The current Docker Engine API runner rejects configured build secrets."
                    onChange={(buildSecrets) => set({ buildSecrets })}
                  />
                ),
              }),
            ],
          }),
        ],
      }),
    }),
    [
      currentGitRepositoryId,
      disabled,
      gitBranches,
      isFetchingGitBranches,
      isFetchingGitRefs,
      isGitBranchesError,
      isLoadingPlatforms,
      mode,
      original.branch,
      platformOptions,
      platforms,
      platformsData,
      selectedPlatform?.connectorType,
      update.branch,
    ],
  );

  return (
    <FormShell<BuildInput>
      key={`${resource?.id ?? 'new'}:${metadataChanged ? 'metadata' : 'base'}`}
      title=""
      schema={schema}
      original={original}
      update={update}
      setUpdate={setUpdate}
      onSave={handleSave}
      disabled={disabled}
      pending={isPending}
      mode={mode}
      draftKey={`Build:${id ?? 'new'}`}
      draftVersion={resource?.rowVersion ?? 'new'}
    />
  );
};

function normalizePayload(payload: BuildInput, mode: 'add' | 'edit') {
  const normalized = {
    ...payload,
    description: payload.description ?? null,
    branch: payload.branch || 'main',
    contextPath: payload.contextPath || '.',
    dockerfilePath: payload.dockerfilePath || 'Dockerfile',
    target: payload.target || null,
    buildArgs: payload.buildArgs ?? [],
    buildSecrets: payload.buildSecrets ?? [],
    tagTemplates: payload.tagTemplates?.filter(Boolean) ?? ['{branch}-{shortSha}'],
    webhook: normalizeWebhook(payload.webhook),
    timeoutSeconds: Number(payload.timeoutSeconds ?? 1800),
    retentionRunCount: Number(payload.retentionRunCount ?? 20),
  };

  if (mode === 'edit') {
    delete (normalized as Partial<BuildProjectInput>).name;
    delete (normalized as Partial<BuildProjectInput>).tagIds;
  }

  return normalized;
}

function normalizeWebhook(webhook: BuildWebhookConfig | undefined | null): BuildWebhookConfig {
  if (!webhook || webhook.enabled !== true) return { enabled: false };

  return {
    ...webhook,
    secret: webhook.secret?.trim() || null,
    branchFilter: webhook.branchFilter?.trim() || null,
  };
}

function mapBranchOptions(
  discoveredBranches: GitRepositoryBranchView[],
  syncedRefs: GitRepositoryRefView[],
): GitBranchOption[] {
  if (discoveredBranches.length > 0) {
    return discoveredBranches.map((branch) => ({
      branch: branch.branch,
      commitSha: branch.commitSha,
    }));
  }

  return syncedRefs.map((ref) => ({
    branch: ref.branch,
    commitSha: ref.resolvedCommitSha,
    lastError: ref.lastError,
  }));
}

function formatJson(value: unknown) {
  try {
    return JSON.stringify(value ?? [], null, 2);
  } catch {
    return '[]';
  }
}

function parseJsonArrayWithDiagnostics<T>(value: string): { value?: T[]; diagnostics: MonacoDiagnostic[] } {
  try {
    const parsed = JSON.parse(value || '[]');
    if (!Array.isArray(parsed)) {
      return {
        diagnostics: [
          {
            lineNumber: 1,
            startColumn: 1,
            message: 'Value must be a JSON array.',
          },
        ],
      };
    }

    return { value: parsed as T[], diagnostics: [] };
  } catch (error) {
    return {
      diagnostics: [
        {
          lineNumber: 1,
          startColumn: 1,
          message: error instanceof Error ? error.message : 'Invalid JSON.',
        },
      ],
    };
  }
}

function validateJsonArray(value: unknown) {
  return Array.isArray(value) ? null : 'Must be a JSON array';
}

function validateRepoRelativePath(value: unknown) {
  const path = String(value ?? '').trim();
  if (!path) return 'Path is required.';
  if (path.includes('\0')) return 'Path is invalid.';
  if (/^[a-zA-Z]:[\\/]/.test(path) || path.startsWith('/') || path.startsWith('\\')) {
    return 'Use a path relative to the repository root.';
  }

  const segments = path.replace(/\\/g, '/').split('/');
  if (segments.some((segment) => segment === '..')) return 'Path cannot leave the repository.';
  return null;
}
