import {
  BuildArgSpec,
  BuildWebhookConfig,
  GitRepositoryBranchView,
  GitRepositoryRefView,
  BuildProjectInput,
  BuildProjectView,
  BuildAgentPoolView,
  BuildProjectBuilderKind,
  BuildSecretSpec,
  LookupResourceType,
  PlatformConnectorType,
  SecretDefinitionView,
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
import { Button } from '@/components/ui/button';
import { useQueryClient } from '@tanstack/react-query';
import { GitBranch, Plus, Trash2 } from 'lucide-react';
import { type ReactNode, useCallback, useMemo, useState } from 'react';
import { useParams } from 'react-router';
import { useBuildRunQuery } from '../hooks/useBuildRunQuery';

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
  builderKind: BuildProjectBuilderKind.Platform,
  platformId: '',
  buildAgentPoolId: null,
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
const buildKitSecretIdPattern = /^[A-Za-z0-9._-]+$/;

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

const BuildPoolField = ({
  value,
  pools,
  isLoading,
  disabled,
  onChange,
}: {
  value?: string | null;
  pools: BuildAgentPoolView[];
  isLoading?: boolean;
  disabled?: boolean;
  onChange: (poolId: string) => void;
}) => {
  const enabledPools = pools.filter((pool) => pool.enabled);
  const selected = pools.find((pool) => pool.id === value);
  const options = enabledPools.map((pool) => ({
    value: pool.id,
    label: (
      <span className="flex min-w-0 flex-col">
        <span className="truncate">{pool.name}</span>
        <span className="truncate text-xs text-muted-foreground">{pool.provider}</span>
      </span>
    ),
  }));
  const hasValidSelection = options.some((option) => option.value === value);
  const hasArchivedSelection = Boolean(value) && !hasValidSelection && Boolean(selected);

  return (
    <div className="flex flex-col gap-2">
      <FieldSelect
        value={hasValidSelection ? (value ?? undefined) : undefined}
        options={options}
        disabled={disabled || isLoading || options.length === 0}
        placeholder={isLoading ? 'Loading build pools...' : 'Select build pool'}
        onChange={onChange}
      />
      <p className={hasArchivedSelection ? 'text-xs text-destructive' : 'text-xs text-muted-foreground'}>
        {hasArchivedSelection
          ? 'This build pool is no longer enabled. Select another pool before saving.'
          : options.length === 0 && !isLoading
            ? 'No enabled build pools are available. Add a Build Pool before selecting this runner.'
            : 'External builder pool that will run this build when the pool runtime is enabled.'}
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

const BuildSecretsField = ({
  value,
  secrets,
  isLoading,
  disabled,
  onChange,
}: {
  value?: BuildSecretSpec[] | null;
  secrets: SecretDefinitionView[];
  isLoading?: boolean;
  disabled?: boolean;
  onChange: (value: BuildSecretSpec[]) => void;
}) => {
  const buildSecrets = value ?? [];
  const secretOptions = useMemo(
    () =>
      secrets.map((secret) => ({
        value: secret.id,
        label: (
          <span className="flex min-w-0 flex-col">
            <span className="truncate">{secret.name}</span>
            <span className="truncate text-xs text-muted-foreground">{secret.providerType}</span>
          </span>
        ),
      })),
    [secrets],
  );

  const setSecret = (index: number, patch: Partial<BuildSecretSpec>) => {
    onChange(
      buildSecrets.map((secret, currentIndex) =>
        currentIndex === index
          ? {
              ...secret,
              ...patch,
            }
          : secret,
      ),
    );
  };

  const removeSecret = (index: number) => {
    onChange(buildSecrets.filter((_, currentIndex) => currentIndex !== index));
  };

  return (
    <div className="flex flex-col gap-3">
      {buildSecrets.length === 0 ? (
        <p className="text-xs text-muted-foreground">
          Add secrets only when your Dockerfile uses BuildKit secret mounts such as
          {' '}
          <span className="font-mono">RUN --mount=type=secret,id=npmrc</span>.
        </p>
      ) : null}

      {buildSecrets.map((secret, index) => {
        const hasSecretSelection = secretOptions.some((option) => option.value === secret.secretId);
        return (
          <div
            key={`build-secret-${index}`}
            className="grid gap-3 border-b pb-3 last:border-b-0 md:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)_auto]">
            <div className="flex min-w-0 flex-col gap-1">
              <span className="text-xs font-medium text-muted-foreground">BuildKit ID</span>
              <FieldInput
                value={secret.id}
                disabled={disabled}
                onChange={(id) => setSecret(index, { id })}
                placeholder="e.g. npmrc"
                className="w-full max-w-full"
              />
              {secret.id && !buildKitSecretIdPattern.test(secret.id) ? (
                <p className="text-xs text-destructive">Use letters, numbers, dot, underscore, or dash.</p>
              ) : (
                <p className="text-xs text-muted-foreground">This must match the id used in the Dockerfile.</p>
              )}
            </div>
            <div className="flex min-w-0 flex-col gap-1">
              <span className="text-xs font-medium text-muted-foreground">Citadel Secret</span>
              <FieldSelect
                value={hasSecretSelection ? secret.secretId : undefined}
                options={secretOptions}
                disabled={disabled || isLoading || secretOptions.length === 0}
                placeholder={isLoading ? 'Loading secrets...' : 'Select secret'}
                onChange={(secretId) => setSecret(index, { secretId })}
                className="w-full max-w-full"
              />
              {!isLoading && secretOptions.length === 0 ? (
                <p className="text-xs text-amber-600">Create a Citadel secret before adding build secrets.</p>
              ) : secret.secretId && !hasSecretSelection ? (
                <p className="text-xs text-amber-600">Selected secret is no longer available.</p>
              ) : (
                <p className="text-xs text-muted-foreground">Secret value is resolved only when the run starts.</p>
              )}
            </div>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="mt-5"
              disabled={disabled}
              onClick={() => removeSecret(index)}
              title="Remove build secret">
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
            ...buildSecrets,
            {
              id: '',
              secretId: '',
            },
          ])
        }>
        <Plus className="size-4" />
        Add Build Secret
      </Button>
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
  const { runId: urlRunId, hash: urlHash, clearRunId } = useBuildRunQuery();
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
  const { data: buildPoolsData, isLoading: isLoadingBuildPools } = useRead('listBuildAgentPools');
  const { data: secretsData, isLoading: isLoadingSecrets } = useRead('listSecretDefinitions');
  const gitRefs = useMemo(() => gitRefsData?.data.refs ?? [], [gitRefsData?.data.refs]);
  const gitBranches = useMemo(
    () => mapBranchOptions(gitBranchesData?.data.branches ?? [], gitRefs),
    [gitBranchesData?.data.branches, gitRefs],
  );
  const platforms = useMemo(() => platformsData?.data.platforms ?? [], [platformsData?.data.platforms]);
  const buildPools = useMemo(() => buildPoolsData?.data.pools ?? [], [buildPoolsData?.data.pools]);
  const secrets = useMemo(() => secretsData?.data.secrets ?? [], [secretsData?.data.secrets]);
  const currentBuilderKind = update.builderKind ?? resource?.builderKind ?? defaultBuild.builderKind;
  const currentPlatformId = update.platformId ?? resource?.platformId ?? defaultBuild.platformId;
  const fallbackPlatformId = currentPlatformId || resource?.platformId || platforms[0]?.id || '';
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
  const handleConfigSave = useCallback(
    async (payload: BuildInput) => {
      if (urlRunId || urlHash === '#runs') {
        clearRunId({ hash: urlHash === '#runs' ? '#config' : undefined });
      }

      await handleSave(payload);
    },
    [clearRunId, handleSave, urlHash, urlRunId],
  );

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
      builderKind: resource.builderKind ?? BuildProjectBuilderKind.Platform,
      buildAgentPoolId: resource.buildAgentPoolId ?? null,
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
                key: 'builderKind',
                label: 'Builder',
                description: 'Choose where Docker builds are executed.',
                required: true,
                render: (value, set) => (
                  <FieldSelect
                    value={value ?? BuildProjectBuilderKind.Platform}
                    disabled={disabled}
                    onChange={(builderKind) =>
                      set({
                        builderKind: builderKind as BuildProjectBuilderKind,
                        platformId: builderKind === BuildProjectBuilderKind.Platform ? fallbackPlatformId : null,
                        buildAgentPoolId:
                          builderKind === BuildProjectBuilderKind.BuildAgentPool
                            ? (update.buildAgentPoolId ?? resource?.buildAgentPoolId ?? null)
                            : null,
                      } as any)
                    }
                    options={[
                      { value: BuildProjectBuilderKind.Platform, label: 'Docker platform' },
                      { value: BuildProjectBuilderKind.BuildAgentPool, label: 'Build pool' },
                    ]}
                  />
                ),
              }),
              ...(currentBuilderKind === BuildProjectBuilderKind.BuildAgentPool
                ? [
                    defineField<BuildInput, 'buildAgentPoolId'>({
                      key: 'buildAgentPoolId',
                      label: 'Build pool',
                      description: 'External builder pool that will run this build.',
                      required: true,
                      validate: (value) =>
                        value && buildPools.some((pool) => pool.enabled && pool.id === value)
                          ? null
                          : 'Select an enabled build pool.',
                      render: (value, set) => (
                        <BuildPoolField
                          value={value}
                          pools={buildPools}
                          isLoading={isLoadingBuildPools}
                          disabled={disabled}
                          onChange={(buildAgentPoolId) => set({ buildAgentPoolId })}
                        />
                      ),
                    }),
                  ]
                : [
                    defineField<BuildInput, 'platformId'>({
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
                  ]),
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
                description:
                  'Map BuildKit secret ids to existing Citadel secrets. Do not put raw secret values in build arguments.',
                validate: (value) => validateBuildSecrets(value, secrets, isLoadingSecrets),
                render: (value, set) => (
                  <BuildSecretsField
                    value={value ?? []}
                    secrets={secrets}
                    isLoading={isLoadingSecrets}
                    disabled={disabled}
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
      currentBuilderKind,
      fallbackPlatformId,
      disabled,
      buildPools,
      gitBranches,
      id,
      isLoadingBuildPools,
      isFetchingGitBranches,
      isFetchingGitRefs,
      isGitBranchesError,
      isLoadingPlatforms,
      isLoadingSecrets,
      mode,
      original.branch,
      platformOptions,
      platforms,
      platformsData,
      secrets,
      selectedPlatform?.connectorType,
      resource?.buildAgentPoolId,
      update.branch,
      update.buildAgentPoolId,
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
      onSave={handleConfigSave}
      disabled={disabled}
      pending={isPending}
      mode={mode}
      draftKey={`Build:${id ?? 'new'}`}
      draftVersion={resource?.rowVersion ?? 'new'}
    />
  );
};

function normalizePayload(payload: BuildInput, mode: 'add' | 'edit') {
  const builderKind = payload.builderKind ?? BuildProjectBuilderKind.Platform;
  const normalized = {
    ...payload,
    description: payload.description ?? null,
    builderKind,
    platformId: builderKind === BuildProjectBuilderKind.Platform ? payload.platformId : null,
    buildAgentPoolId: builderKind === BuildProjectBuilderKind.BuildAgentPool ? payload.buildAgentPoolId : null,
    branch: payload.branch || 'main',
    contextPath: payload.contextPath || '.',
    dockerfilePath: payload.dockerfilePath || 'Dockerfile',
    target: payload.target || null,
    buildArgs: payload.buildArgs ?? [],
    buildSecrets:
      payload.buildSecrets
        ?.map((secret) => ({
          id: secret.id.trim(),
          secretId: secret.secretId,
        }))
        .filter((secret) => secret.id || secret.secretId) ?? [],
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

function validateBuildSecrets(value: unknown, secrets: SecretDefinitionView[], isLoadingSecrets: boolean) {
  if (!Array.isArray(value)) return 'Build secrets must be a list.';

  const usedIds = new Set<string>();
  for (const entry of value as BuildSecretSpec[]) {
    const id = entry.id?.trim();
    if (!id) return 'BuildKit secret id is required.';
    if (!buildKitSecretIdPattern.test(id)) return `BuildKit secret id "${id}" is invalid.`;
    const normalizedId = id.toLowerCase();
    if (usedIds.has(normalizedId)) return `BuildKit secret id "${id}" is mapped more than once.`;
    usedIds.add(normalizedId);

    if (!entry.secretId) return `Select a Citadel secret for "${id}".`;
    if (!isLoadingSecrets && !secrets.some((secret) => secret.id === entry.secretId)) {
      return `Citadel secret for "${id}" is no longer available.`;
    }
  }

  return null;
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
