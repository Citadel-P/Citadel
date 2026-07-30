import {
  CreateDeploymentInput,
  PlatformView,
  ImageView,
  DeploymentImageInfoBuildImage,
  DeploymentImageInfoExternalImage,
  DeploymentImageInfoLocalImage,
  DockerNetworkResultView,
  ContainerRestartPolicy,
  ResourceSpec,
  StopSignal,
  UpdateBehavior,
  DeploymentConfigView,
  PatchDeploymentInput,
  LookupResourceType,
  BuildRunStatus,
  LicenseCapability,
  AdoptionIssueSeverity,
  AdoptContainerInput,
  type ContainerAdoptionIssueView,
} from '@/api/generated/api.types';
import {
  FormShell,
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldTextArea,
  defineRowField,
  PortMappingField,
  ItemSelector,
  FieldSelect,
  FieldSwitch,
} from '@/components/custom/form-builder';
import { useState, useMemo, useEffect, useCallback, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useConfirmByName, useDialogHotkeys, useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { useParams, useSearchParams } from 'react-router';
import { MultiResourceSelectorField, ResourceSelectorField } from '@/components/custom/common';
import { MonacoToArrayEditor, MonacoToDictionaryEditor } from '@/lib/monaco';
import { AlertMessage } from '@/components/custom/alert-message';
import { ResourceTagSelector } from '@/features/tags/components';
import { BuildImageProvenanceStatus } from '@/features/builds/build-image-provenance-status';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import { ConfirmButton } from '@/components/custom/action-with-dialog';
import { KeyRound, PackagePlus } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { toast } from 'sonner';
import { getDeploymentConfigurationNames } from './adoption-configuration-names';
import { shouldApplyLocalImagePortDefaults } from './image-port-defaults';

const enum ImageSource {
  local = 'Local',
  external = 'External',
  build = 'Build',
}

const enum ResourceProfile {
  automatic,
  xsmall,
  small,
  medium,
  large,
  xlarge,
}

const update_behaviors = {
  [UpdateBehavior.Disabled]: {
    label: 'Disabled',
    description: 'Do not check for updates.',
  },
  [UpdateBehavior.Notify]: {
    label: 'Notify Only',
    description: 'Periodically check for updates and alert me, but do not redeploy.',
  },
  [UpdateBehavior.AutoDeploy]: {
    label: 'Auto Deploy',
    description: 'Periodically check and automatically redeploy when a new image is found.',
  },
};

const image_source = {
  [ImageSource.local]: {
    label: 'Local',
    description: 'Use an image that already exists on the target Docker host.',
  },
  [ImageSource.external]: {
    label: 'External',
    description: 'Pull an image from a remote registry (Docker Hub, GitHub, etc.)',
  },
  [ImageSource.build]: {
    label: 'Build',
    description: 'Use the latest successful image produced by a Citadel build.',
  },
};

const restart_policies = {
  No: { label: 'No', description: 'Do not automatically restart' },
  Always: {
    label: 'Always',
    description: 'Automatically restarts the container whenever it stops.',
  },
  UnlessStopped: {
    label: 'Unless Stopped',
    description: 'Restart always except when the user has manually stopped the container',
  },
  OnFailure: {
    label: 'On Failure',
    description: 'Restart only when the container exit code is non-zero',
  },
};

const resource_profiles = {
  [ResourceProfile.automatic]: {
    label: 'Automatic',
    description: 'Resources will be allocated automatically by the platform',
    spec: null,
  },
  [ResourceProfile.xsmall]: {
    label: 'X-Small',
    description: '0.25 CPU - 256 MB RAM',
    spec: { nanoCpus: 0.25, memoryLimit: 256 },
  },
  [ResourceProfile.small]: {
    label: 'Small',
    description: '0.5 CPU - 512 MB RAM',
    spec: { nanoCpus: 0.5, memoryLimit: 512 },
  },
  [ResourceProfile.medium]: {
    label: 'Medium',
    description: '0.5 CPU - 1 GB RAM',
    spec: { nanoCpus: 0.5, memoryLimit: 1024 },
  },
  [ResourceProfile.large]: {
    label: 'Large',
    description: '1.0 CPU - 2 GB RAM',
    spec: { nanoCpus: 1, memoryLimit: 2048 },
  },
  [ResourceProfile.xlarge]: {
    label: 'X-Large',
    description: '2.0 CPU - 4 GB RAM',
    spec: { nanoCpus: 2, memoryLimit: 4096 },
  },
};

const stop_signals = {
  [StopSignal.SIGTERM]: {
    label: StopSignal.SIGTERM,
    description: 'Request a graceful shutdown. Default and recommended.',
  },
  [StopSignal.SIGINT]: {
    label: StopSignal.SIGINT,
    description: 'Gracefully interrupt the process (similar to Ctrl+C).',
  },
  [StopSignal.SIGKILL]: {
    label: StopSignal.SIGKILL,
    description: 'Forcefully stop the process immediately.',
  },
  [StopSignal.SIGQUIT]: {
    label: StopSignal.SIGQUIT,
    description: 'Request a graceful shutdown when supported by the application.',
  },
};

type DeploymentInput = CreateDeploymentInput | PatchDeploymentInput;

const EMPTY_RESOURCE_BINDING_LOOKUP: { name: string }[] = [];
const EMPTY_ADOPTION_ISSUES: ContainerAdoptionIssueView[] = [];
const environmentNamePattern = /^[A-Za-z_][A-Za-z0-9_]*$/;
const environmentReferencePattern = /\$\{([^}]+)\}/g;

const validateDeploymentEnvironmentVariable = (line: string, configurationNames: string[]) => {
  const separator = line.indexOf('=');
  const key = (separator < 0 ? line : line.slice(0, separator)).trim();

  if (!environmentNamePattern.test(key)) {
    return `${key || 'Environment key'} is not a valid environment key.`;
  }

  const knownNames = new Set(configurationNames);
  if (separator < 0) {
    return knownNames.has(key) ? null : `${key} is not defined in deployment or global variables.`;
  }

  const value = line.slice(separator + 1);
  for (const match of value.matchAll(environmentReferencePattern)) {
    const name = match[1]?.trim() ?? '';
    if (!environmentNamePattern.test(name)) {
      return `${match[0]} is not a supported variable reference.`;
    }

    if (!knownNames.has(name)) {
      return `${name} is not defined in deployment or global variables.`;
    }
  }

  return null;
};

export const DeploymentForm = ({
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
  const adoptFrom = mode === 'add' ? searchParams.get('adoptFrom') : null;
  const duplicateDraftLoadedRef = useRef<string | null>(null);
  const adoptionDraftLoadedRef = useRef<string | null>(null);
  const adoptionPreviewFingerprintRef = useRef<string | null>(null);
  const adoptionConfirmationResolver = useRef<((confirmed: boolean) => void) | null>(null);
  const adoptionConfirmButtonRef = useRef<HTMLButtonElement>(null);
  const [adoptionConfirmationOpen, setAdoptionConfirmationOpen] = useState(false);
  const [sensitiveImportPreference, setSensitiveImportPreference] = useState<{
    containerId: string;
    enabled: boolean;
  } | null>(null);
  const [update, setUpdate] = useState<Partial<DeploymentInput>>({});
  const queryClient = useQueryClient();
  const { hasCapability: hasLicenseCapability } = useLicenseEntitlements();
  const automatedOperationsEnabled = hasLicenseCapability(LicenseCapability.AutomatedOperations);
  const operationalGuardrailsEnabled = hasLicenseCapability(LicenseCapability.OperationalGuardrails);

  const { mutateAsync: createDeployment } = useMutate('createDeployment');
  const { mutateAsync: adoptContainer } = useMutate('adoptContainer');
  const { mutateAsync: updateDeployment } = useMutate('updateDeployment');
  const { data: deploymentCfg } = useRead('getDeploymentConfig', { deploymentId: id });
  const { data: buildProjectsData, isFetching: buildProjectsLoading } = useRead('listBuildProjects');
  const { data: duplicateDraftData, isFetching: isDuplicateDraftLoading } = useRead(
    'getDeploymentDuplicateDraft',
    { deploymentId: duplicateFrom ?? '' },
    { enabled: mode === 'add' && !!duplicateFrom },
  );
  const { data: adoptionDraftData, isFetching: isAdoptionDraftLoading } = useRead(
    'getContainerAdoptionDraft',
    { id: adoptFrom ?? '' },
    { enabled: mode === 'add' && !!adoptFrom },
  );
  const { data: resourceBindingLookupData } = useRead('lookup', {
    query: {
      TargetResourceType: LookupResourceType.ResourceBinding,
      SourceResourceType: mode === 'edit' ? LookupResourceType.Deployment : undefined,
      SourceResourceId: mode === 'edit' ? id : undefined,
    },
  });

  const resource: DeploymentConfigView | undefined = deploymentCfg?.data;
  const duplicateDraft = duplicateDraftData?.data;
  const adoptionDraft = adoptionDraftData?.data;
  const duplicateWarnings = duplicateDraft?.warnings ?? [];
  const adoptionIssues = adoptionDraft?.issues ?? EMPTY_ADOPTION_ISSUES;
  const sensitiveAdoptionIssues = adoptionIssues.filter(
    (issue) => issue.code === 'SENSITIVE_ENVIRONMENT_VALUE_REQUIRED',
  );
  const importSensitiveEnvironmentAsSecrets =
    adoptionDraft?.canImportSensitiveEnvironmentValues === true &&
    (sensitiveImportPreference?.containerId === adoptFrom ? sensitiveImportPreference.enabled : true);
  const visibleAdoptionIssues = importSensitiveEnvironmentAsSecrets
    ? adoptionIssues.filter((issue) => issue.code !== 'SENSITIVE_ENVIRONMENT_VALUE_REQUIRED')
    : adoptionIssues;
  const adoptionContainerName = adoptionDraft?.source.name?.replace(/^\/+/, '') ?? '';
  const hasAdoptionBlocker = adoptionIssues.some((issue) => issue.severity === AdoptionIssueSeverity.Blocker);
  const resolveAdoptionConfirmation = useCallback((confirmed: boolean) => {
    const resolver = adoptionConfirmationResolver.current;
    adoptionConfirmationResolver.current = null;
    setAdoptionConfirmationOpen(false);
    resolver?.(confirmed);
  }, []);
  const {
    input: adoptionConfirmationInput,
    setInput: setAdoptionConfirmationInput,
    isLoading: adoptionConfirmationLoading,
    isConfirmDisabled: adoptionConfirmationDisabled,
    handleConfirm: handleAdoptionConfirmation,
    reset: resetAdoptionConfirmation,
  } = useConfirmByName({
    name: adoptionContainerName,
    disabled: !adoptionContainerName,
    onConfirm: () => resolveAdoptionConfirmation(true),
    onClose: () => setAdoptionConfirmationOpen(false),
    hotkeysEnabled: adoptionConfirmationOpen,
  });
  useDialogHotkeys({
    enabled: adoptionConfirmationOpen,
    onConfirm: handleAdoptionConfirmation,
    onCancel: () => resolveAdoptionConfirmation(false),
    confirmDisabled: adoptionConfirmationDisabled,
    confirmButtonRef: adoptionConfirmButtonRef,
  });
  const confirmAdoption = useCallback(
    () =>
      new Promise<boolean>((resolve) => {
        resetAdoptionConfirmation();
        adoptionConfirmationResolver.current = resolve;
        setAdoptionConfirmationOpen(true);
      }),
    [resetAdoptionConfirmation],
  );
  const formDraftKey = adoptFrom
    ? `deployment:adopt:${adoptFrom}`
    : duplicateFrom
      ? `deployment:duplicate:${duplicateFrom}`
      : `deployment:${id ?? 'new'}`;

  const original = resource ?? ({} as DeploymentConfigView);

  // Fallback Logic: Check `update` first, then `original`.
  const currentPlatformId = update.platformId ?? original.platformId;
  const currentSpec = { ...original.spec, ...update.spec };
  const currentImage = update.spec?.image ?? original.spec?.image;
  const provider = currentImage?.$type;
  const licensedUpdateBehaviors = useMemo(
    () => ({
      ...update_behaviors,
      [UpdateBehavior.AutoDeploy]: {
        ...update_behaviors[UpdateBehavior.AutoDeploy],
        label: 'Auto Deploy',
        disabled: !operationalGuardrailsEnabled,
        requiredLicense: !operationalGuardrailsEnabled ? ('Team' as const) : undefined,
      },
    }),
    [operationalGuardrailsEnabled],
  );
  const buildProjects = useMemo(() => buildProjectsData?.data.projects ?? [], [buildProjectsData?.data.projects]);
  const buildProjectOptions = useMemo(
    () =>
      buildProjects.map((project) => ({
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
    [buildProjects],
  );
  const effectiveResourceBindings = resourceBindingLookupData?.data ?? EMPTY_RESOURCE_BINDING_LOOKUP;
  const effectiveConfigurationNames = useMemo(
    () =>
      getDeploymentConfigurationNames(
        effectiveResourceBindings.map((entry) => entry.name),
        adoptionIssues,
        importSensitiveEnvironmentAsSecrets,
      ),
    [effectiveResourceBindings, adoptionIssues, importSensitiveEnvironmentAsSecrets],
  );

  const localImageId = (currentImage as DeploymentImageInfoLocalImage | undefined)?.imageId;
  const requestedImagePortDefaultsRef = useRef<string | null>(null);
  const shouldLoadImagePorts =
    !adoptFrom && currentImage?.$type === ImageSource.local && !!currentPlatformId && !!localImageId;
  const { data, isSuccess: imageInfoIsSuccess } = useRead(
    'getExposedPorts',
    {
      platformId: currentPlatformId,
      imageId: localImageId,
    },
    { enabled: shouldLoadImagePorts },
  );

  useEffect(() => {
    const requestedImageId = requestedImagePortDefaultsRef.current;
    const hasResponse =
      imageInfoIsSuccess && data?.data?.ports !== undefined && currentImage?.$type === ImageSource.local;
    if (!hasResponse || requestedImageId !== localImageId) return;

    const serverPorts = data.data.ports ?? [];
    const userPorts = update.spec?.ports;
    requestedImagePortDefaultsRef.current = null;
    if (!shouldApplyLocalImagePortDefaults(requestedImageId, localImageId, userPorts)) return;

    setUpdate(
      (prev) =>
        ({
          ...prev,
          spec: {
            ...(prev.spec ?? {}),
            ports: serverPorts,
          },
        }) as Partial<DeploymentInput>,
    );
  }, [imageInfoIsSuccess, data?.data?.ports, currentImage?.$type, localImageId, update.spec?.ports]);

  useEffect(() => {
    if (!duplicateFrom || !duplicateDraft?.draft || duplicateDraftLoadedRef.current === duplicateFrom) return;

    duplicateDraftLoadedRef.current = duplicateFrom;
    setUpdate(duplicateDraft.draft as Partial<DeploymentInput>);
  }, [duplicateFrom, duplicateDraft?.draft]);

  useEffect(() => {
    if (!adoptFrom || !adoptionDraft?.draft || adoptionDraftLoadedRef.current === adoptFrom) return;

    adoptionDraftLoadedRef.current = adoptFrom;
    adoptionPreviewFingerprintRef.current = adoptionDraft.previewFingerprint;
    setUpdate(adoptionDraft.draft as Partial<DeploymentInput>);
  }, [adoptFrom, adoptionDraft?.draft, adoptionDraft?.previewFingerprint]);

  const refreshData = useCallback(() => {
    localStorage.removeItem(formDraftKey);
    queryClient.invalidateQueries({ queryKey: ['getDeploymentConfig', { deploymentId: id }] });
  }, [formDraftKey, id, queryClient]);

  useEffect(() => {
    if (!metadataChanged) return;
    refreshData();
  }, [metadataChanged, refreshData]);

  const { save: handleSave, isPending } = useSaveResource<DeploymentInput, any>({
    mode,
    basePath: 'deployments',
    entityName: 'Deployment',
    onCreate: (payload) => {
      if (adoptFrom && adoptionDraft) {
        const createPayload = payload as CreateDeploymentInput;
        const previewFingerprint = adoptionPreviewFingerprintRef.current;
        if (!previewFingerprint) {
          return Promise.reject(new Error('Reload the container adoption draft before continuing.'));
        }

        const adoptionPayload: AdoptContainerInput = {
          name: createPayload.name,
          description: createPayload.description,
          spec: createPayload.spec,
          previewFingerprint,
          tagIds: createPayload.tagIds,
          importSensitiveEnvironmentAsSecrets,
        };
        return adoptContainer({ id: adoptFrom, data: adoptionPayload }).then(async (response) => {
          await Promise.all([
            queryClient.invalidateQueries({ queryKey: ['listContainers'] }),
            queryClient.invalidateQueries({ queryKey: ['getContainerData'] }),
          ]);
          return response;
        });
      }

      return createDeployment({ data: payload as CreateDeploymentInput });
    },
    onUpdate: (payload) => updateDeployment({ id, data: payload }),
    onRefresh: refreshData,
  });

  const schema = useMemo(
    () => ({
      general: defineSection<DeploymentInput>({
        title: '',
        items: [
          ...(mode === 'add'
            ? [
                defineGroupField<DeploymentInput>({
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
                        <FieldInput
                          value={val}
                          onChange={(v) => set({ name: v })}
                          placeholder="e.g. production-web-server"
                        />
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
                      description: 'Optional tags for filtering and grouping this deployment.',
                      render: (val, set) => (
                        <ResourceTagSelector value={val} disabled={disabled} onChange={(tagIds) => set({ tagIds })} />
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
            disabled: !!adoptFrom,
            description: 'Select the platform to deploy on.',
            render: (value, set) => {
              return (
                <ResourceSelectorField
                  sourceType={LookupResourceType.Deployment}
                  targetType={LookupResourceType.Platform}
                  sourceResourceId={id}
                  selected={value}
                  onSelect={(v: PlatformView | undefined) => set({ platformId: v?.id })}
                  placeholder="Select Platform"
                />
              );
            },
          }),
          defineGroupField<DeploymentInput>({
            id: 'image',
            label: 'Image',
            items: [
              defineField({
                label: 'Image Source',
                key: 'spec.image.$type',
                description: 'Select the image source.',
                required: true,
                validate: (v) => (!v ? 'Source is required' : null),
                render: (val, set) => {
                  return (
                    <ItemSelector
                      collection={image_source}
                      value={val}
                      onChange={(v: ImageSource) =>
                        set((prev) => ({
                          spec: {
                            ...(prev.spec as DeploymentInput['spec']),
                            image:
                              v === ImageSource.build
                                ? ({ $type: 'Build', buildProjectId: '', redeployOnBuild: false } as any)
                                : ({ $type: v } as any),
                            updateBehavior:
                              v === ImageSource.build ? UpdateBehavior.Disabled : prev.spec?.updateBehavior,
                            ports: [],
                          },
                        }))
                      }
                    />
                  );
                },
              }),

              ...(provider === ImageSource.external
                ? [
                    defineRowField<DeploymentInput>({
                      id: 'imageRow',
                      gap: 'gap-8',
                      fields: [
                        defineField<DeploymentInput, 'spec.image.registryId'>({
                          key: 'spec.image.registryId',
                          label: 'Registry',
                          required: true,
                          description: 'Select the registry to pull the image from.',
                          render: (val, set) => {
                            return (
                              <ResourceSelectorField
                                sourceType={LookupResourceType.Deployment}
                                targetType={LookupResourceType.Registry}
                                sourceResourceId={id}
                                selected={val}
                                onSelect={(v: ImageView | undefined) =>
                                  set((prev) => ({
                                    spec: {
                                      ...prev.spec!,
                                      image: {
                                        $type: 'External',
                                        ...((prev.spec?.image as DeploymentImageInfoExternalImage) ?? {}),
                                        registryId: v?.id ?? '',
                                      } satisfies DeploymentImageInfoExternalImage,
                                    },
                                  }))
                                }
                                placeholder="Select Registry"
                                className="sm:min-w-100"
                              />
                            );
                          },
                        }),
                        defineField<DeploymentInput, 'spec.image.imageTag'>({
                          key: 'spec.image.imageTag',
                          label: ' ',
                          required: true,
                          description: 'Enter the image reference.',
                          render: (val, set) => {
                            return (
                              <FieldInput
                                value={val}
                                onChange={(v) =>
                                  set((prev) => ({
                                    spec: {
                                      ...prev.spec!,
                                      image: {
                                        $type: 'External',
                                        ...((prev.spec?.image as DeploymentImageInfoExternalImage) ?? {}),
                                        imageTag: v,
                                      } satisfies DeploymentImageInfoExternalImage,
                                    },
                                  }))
                                }
                                placeholder="e.g. nginx:latest "
                                className="w-full max-w-full"
                              />
                            );
                          },
                        }),
                      ],
                    }),
                  ]
                : provider === ImageSource.build
                  ? [
                      defineField<DeploymentInput, 'spec.image.buildProjectId'>({
                        key: 'spec.image.buildProjectId',
                        label: 'Build',
                        required: true,
                        description: 'Build project whose latest successful image will be deployed.',
                        validate: (v) => {
                          if (!v) return 'Build is required';
                          if (!buildProjectsLoading && !buildProjects.some((project) => project.id === v)) {
                            return 'Selected build is no longer available';
                          }
                          return null;
                        },
                        render: (val, set) => {
                          const hasSelection = buildProjectOptions.some((option) => option.value === val);
                          const selectedProject = buildProjects.find((project) => project.id === val);
                          const latestRun = selectedProject?.latestRun;
                          const latestImageReference = latestRun?.imageReferences?.[0];
                          const hasSuccessfulImage =
                            latestRun?.status === BuildRunStatus.Succeeded && !!latestImageReference;
                          return (
                            <div className="flex flex-col gap-2">
                              <FieldSelect
                                value={hasSelection ? val : undefined}
                                options={buildProjectOptions}
                                disabled={disabled || buildProjectsLoading || buildProjectOptions.length === 0}
                                placeholder={buildProjectsLoading ? 'Loading builds...' : 'Select build'}
                                onChange={(buildProjectId) =>
                                  set((prev) => ({
                                    spec: {
                                      ...prev.spec!,
                                      image: {
                                        $type: 'Build',
                                        ...((prev.spec?.image as DeploymentImageInfoBuildImage) ?? {}),
                                        buildProjectId,
                                      } satisfies DeploymentImageInfoBuildImage,
                                      updateBehavior: UpdateBehavior.Disabled,
                                    },
                                  }))
                                }
                              />
                              {!buildProjectsLoading && buildProjectOptions.length === 0 ? (
                                <AlertMessage type="warning" title="No builds available">
                                  Create a build project before using a build image source.
                                </AlertMessage>
                              ) : val && !selectedProject && !buildProjectsLoading ? (
                                <AlertMessage type="warning" title="Build unavailable">
                                  The selected build no longer exists. Select another build before saving.
                                </AlertMessage>
                              ) : selectedProject && hasSuccessfulImage ? (
                                <BuildImageProvenanceStatus
                                  value={currentImage as DeploymentImageInfoBuildImage}
                                  latestImageReference={latestImageReference}
                                  latestDigest={latestRun?.imageDigest}
                                  latestBuildRunId={latestRun?.id}
                                />
                              ) : selectedProject && latestRun ? (
                                <AlertMessage type="warning" title={`Latest run ${latestRun.status}`}>
                                  Deployment can be saved, but it cannot apply this build image until a run succeeds.
                                </AlertMessage>
                              ) : selectedProject ? (
                                <AlertMessage type="info" title="No build image yet">
                                  Deployment can be saved, but it cannot apply this build image until the first build
                                  succeeds.
                                </AlertMessage>
                              ) : null}
                            </div>
                          );
                        },
                      }),
                      defineField<DeploymentInput, 'spec.image.redeployOnBuild'>({
                        key: 'spec.image.redeployOnBuild',
                        label: 'Redeploy On Build',
                        description: 'Automatically redeploy this deployment after the selected build succeeds.',
                        requiredLicense: automatedOperationsEnabled ? undefined : 'Team',
                        render: (val, set) => (
                          <FieldSwitch
                            id="deployment-redeploy-on-build"
                            checked={val ?? false}
                            disabled={disabled || (!val && !automatedOperationsEnabled)}
                            onChange={(redeployOnBuild) =>
                              set((prev) => ({
                                spec: {
                                  ...prev.spec!,
                                  image: {
                                    $type: 'Build',
                                    ...((prev.spec?.image as DeploymentImageInfoBuildImage) ?? {}),
                                    redeployOnBuild,
                                  } satisfies DeploymentImageInfoBuildImage,
                                  updateBehavior: UpdateBehavior.Disabled,
                                },
                              }))
                            }
                          />
                        ),
                      }),
                    ]
                  : [
                      defineField<DeploymentInput, 'spec.image.imageId'>({
                        key: 'spec.image.imageId',
                        label: `Local Image`,
                        required: true,
                        description: 'These images are immediately available for deployment without a remote pull.',
                        render: (val, set) => (
                          <ResourceSelectorField
                            sourceType={id ? LookupResourceType.Deployment : LookupResourceType.Platform}
                            targetType={LookupResourceType.Image}
                            sourceResourceId={id ?? currentPlatformId}
                            platformId={currentPlatformId}
                            queryEnabled={!!currentPlatformId}
                            selected={val}
                            onSelect={(v: ImageView | undefined) => {
                              const selectedImageId = v?.id ?? '';
                              requestedImagePortDefaultsRef.current = adoptFrom ? null : selectedImageId || null;
                              set((prev) => ({
                                spec: {
                                  ...prev.spec!,
                                  image: {
                                    $type: 'Local',
                                    ...((prev.spec?.image as DeploymentImageInfoLocalImage) ?? {}),
                                    imageId: selectedImageId,
                                  } satisfies DeploymentImageInfoLocalImage,
                                  ports: adoptFrom ? prev.spec?.ports : [],
                                },
                              }));
                            }}
                            placeholder="Select Image"
                          />
                        ),
                      }),
                    ]),
            ],
          }),
          defineGroupField<DeploymentInput>({
            id: 'networks',
            label: 'Networks',
            items: [
              defineField({
                label: 'Networks',
                key: 'spec.networks',
                description: 'Select the Docker networks that this container will attach to.',
                required: true,
                validate: (v) => (!v ? 'Source is required' : null),
                render: (value, set) => (
                  <MultiResourceSelectorField
                    targetType={LookupResourceType.Network}
                    sourceType={LookupResourceType.Deployment}
                    sourceResourceId={mode == 'add' ? undefined : id}
                    platformId={currentPlatformId}
                    queryEnabled={!!currentPlatformId}
                    selected={value ?? []}
                    onSelect={(v: DockerNetworkResultView[] | undefined) =>
                      set((prev) => ({
                        spec: {
                          ...prev.spec!,
                          networks: v?.map((s) => s.name) ?? [],
                        },
                      }))
                    }
                    placeholder="Select Network(s)"
                    valueKey="name"
                  />
                ),
              }),
              provider === ImageSource.local
                ? defineField({
                    label: 'Ports',
                    key: 'spec.ports',
                    description: 'Configure port mappings.',
                    required: false,
                    render: (value, set) => (
                      <PortMappingField
                        ports={value ?? []}
                        set={(v: string[] | undefined) =>
                          set((prev) => ({
                            spec: {
                              ...prev.spec!,
                              ports: v?.map((s) => s) ?? [],
                            },
                          }))
                        }
                      />
                    ),
                  })
                : defineField({
                    label: 'Ports',
                    key: 'spec.ports',
                    description: 'Configure port mappings.',
                    required: false,
                    render: (value, set) => (
                      <MonacoToArrayEditor
                        value={value}
                        helperText="# 8080:8080/tcp"
                        language="key_value"
                        onChange={(p: string[] | undefined) =>
                          set((prev) => ({
                            spec: {
                              ...prev.spec!,
                              ports: p ?? [],
                            },
                          }))
                        }
                      />
                    ),
                  }),
            ],
          }),
          defineField({
            key: 'spec.volumes',
            label: 'Volumes',
            description: 'Configure bind mounts or named volumes.',
            required: false,
            render: (value, set) => (
              <MonacoToArrayEditor
                value={value}
                helperText="# my-volume:/data or /config:/etc/config:ro"
                language="string_list"
                onChange={(v: string[] | undefined) =>
                  set((prev) => ({
                    spec: {
                      ...prev.spec!,
                      volumes: v ?? [],
                    },
                  }))
                }
              />
            ),
          }),
          defineField({
            key: 'spec.environmentVariables',
            label: 'Container Variables',
            description:
              'Select the environment keys injected into the container. Use KEY to expose a matching Citadel variable or KEY=${OTHER_KEY} to map a value.',
            required: false,
            render: (value, set) => (
              <MonacoToArrayEditor
                value={value}
                helperText="# db_password=${POSTGRES_PASSWORD}"
                language="key_value"
                completionItems={effectiveConfigurationNames}
                completionItemDetail="Citadel variable or secret"
                validateItem={(line) => validateDeploymentEnvironmentVariable(line, effectiveConfigurationNames)}
                onChange={(environmentVariables: string[] | undefined) =>
                  set((prev) => ({
                    spec: {
                      ...prev.spec!,
                      environmentVariables: environmentVariables ?? [],
                    },
                  }))
                }
              />
            ),
          }),
          defineField({
            key: 'spec.updateBehavior',
            label: 'Auto Update',
            description: 'Define how the platform handles new image versions.',
            render: (value, set) => {
              let disabled = provider !== ImageSource.external;
              let warningMsg = 'Auto update requires an external image source.';

              if (provider === ImageSource.build) {
                warningMsg = 'Build images use Redeploy On Build instead of registry auto-update.';
              }

              if (
                provider === ImageSource.external &&
                (currentSpec.image as DeploymentImageInfoExternalImage)?.imageTag?.includes('@')
              ) {
                disabled = true;
                warningMsg = "Cannot enable Auto-update for an image pinned by digest (contains '@')";
              }
              return (
                <div className="flex flex-col gap-2">
                  <ItemSelector
                    collection={licensedUpdateBehaviors}
                    value={value}
                    disabled={disabled}
                    onChange={(updateBehavior: UpdateBehavior) => {
                      set((prev) => ({
                        spec: {
                          ...prev.spec!,
                          updateBehavior: updateBehavior,
                        },
                      }));
                    }}
                  />
                  {disabled && (
                    <AlertMessage type={'warning'} title={''}>
                      <span className=" font-normal">{warningMsg}</span>
                    </AlertMessage>
                  )}
                </div>
              );
            },
          }),
        ],
      }),
      Advanced: defineSection<DeploymentInput>({
        title: 'Advanced',
        items: [
          defineField({
            key: 'spec.resourceSpec',
            label: 'Resources',
            description: 'Choose how much CPU and memory to allocate to this deployment.',
            render: (value, set) => {
              const toProfile = (spec: ResourceSpec | undefined) => {
                return (
                  Object.entries(resource_profiles).find(
                    ([_, p]) => p.spec?.nanoCpus === spec?.nanoCpus && p.spec?.memoryLimit === spec?.memoryLimit,
                  )?.[0] ?? 'automatic'
                );
              };
              return (
                <ItemSelector
                  collection={resource_profiles}
                  value={toProfile(value)}
                  onChange={(profile: ResourceProfile) =>
                    set((prev) => ({
                      spec: {
                        ...(prev.spec as DeploymentInput['spec']),
                        resourceSpec: resource_profiles[profile]?.spec ?? null,
                      },
                    }))
                  }
                />
              );
            },
          }),
          defineGroupField({
            id: 'lifecycle',
            label: 'Lifecycle',
            title: 'Lifecycle',
            description: 'Manage container reliability and shutdown behavior.',
            items: [
              defineField({
                key: 'spec.lifeCycleSpec.restartPolicy',
                label: 'Restart Policy',
                description: 'The behavior to apply when the container exits.',
                render: (value, set) => (
                  <ItemSelector
                    collection={restart_policies}
                    value={value}
                    onChange={(policy: ContainerRestartPolicy) =>
                      set((prev) => ({
                        spec: {
                          ...(prev.spec as DeploymentInput['spec']),
                          lifeCycleSpec: {
                            ...(prev.spec?.lifeCycleSpec as any),
                            restartPolicy: policy,
                          },
                        },
                      }))
                    }
                  />
                ),
              }),
              defineField({
                key: 'spec.lifeCycleSpec.stopSignal',
                label: 'Stop Signal',
                description: 'Signal sent to the container to initiate shutdown.',
                render: (value, set) => (
                  <ItemSelector
                    collection={stop_signals}
                    value={value}
                    onChange={(sig: ContainerRestartPolicy) =>
                      set((prev) => ({
                        spec: {
                          ...(prev.spec as DeploymentInput['spec']),
                          lifeCycleSpec: {
                            ...(prev.spec?.lifeCycleSpec as any),
                            stopSignal: sig,
                          },
                        },
                      }))
                    }
                  />
                ),
              }),

              defineField({
                key: 'spec.lifeCycleSpec.stopTimeout',
                label: 'Stop Timeout',
                description:
                  'Maximum time to wait (in seconds) for the container to stop before it is forcefully terminated.',
                render: (val, set) => (
                  <FieldInput
                    type="number"
                    value={val}
                    onChange={(v) =>
                      set((prev) => ({
                        spec: {
                          ...(prev.spec as DeploymentInput['spec']),
                          lifeCycleSpec: {
                            ...(prev.spec?.lifeCycleSpec as any),
                            stopTimeout: v,
                          },
                        },
                      }))
                    }
                    placeholder="e.g. 10"
                  />
                ),
              }),
            ],
          }),
          defineField({
            label: 'Command',
            key: 'spec.command',
            description: 'Overrides the image default command.',
            required: false,
            render: (value, set) => (
              <MonacoToArrayEditor
                value={value}
                helperText="# --housekeeping_interval=5s"
                language="key_value"
                onChange={(cmd: string[] | undefined) =>
                  set((prev) => ({
                    spec: {
                      ...prev.spec!,
                      command: cmd ?? [],
                    },
                  }))
                }
              />
            ),
          }),

          defineField({
            key: 'spec.labels',
            label: 'Labels',
            description: 'User-defined key/value metadata.',
            render: (value, set) => (
              <MonacoToDictionaryEditor
                value={value}
                helperText="# KEY=value"
                language="key_value"
                onChange={(e: Record<string, string> | undefined) =>
                  set((prev) => ({
                    spec: {
                      ...prev.spec!,
                      labels: e ?? {},
                    },
                  }))
                }
              />
            ),
          }),
        ],
      }),
    }),
    [
      provider,
      automatedOperationsEnabled,
      licensedUpdateBehaviors,
      currentPlatformId,
      currentImage,
      currentSpec.image,
      mode,
      id,
      effectiveConfigurationNames,
      disabled,
      adoptFrom,
      buildProjects,
      buildProjectOptions,
      buildProjectsLoading,
    ],
  );

  return (
    <div className="flex flex-col gap-3">
      {duplicateFrom && (
        <AlertMessage type="info" title={isDuplicateDraftLoading ? 'Loading duplicate draft' : 'Duplicate draft'}>
          No deployment has been created yet. Review the copied configuration, then save it to create the deployment.
        </AlertMessage>
      )}
      {duplicateWarnings.map((warning) => (
        <AlertMessage key={`${warning.code}:${warning.fieldPath ?? ''}`} type="warning" title={warning.code}>
          {warning.message}
        </AlertMessage>
      ))}
      {adoptFrom && (
        <AlertMessage type="info" title={isAdoptionDraftLoading ? 'Inspecting container' : 'Adopt container'}>
          Citadel will attach the existing container to this deployment without recreating or restarting it. Review the
          generated configuration before confirming.
        </AlertMessage>
      )}
      {adoptFrom && adoptionDraft?.canImportSensitiveEnvironmentValues && sensitiveAdoptionIssues.length > 0 && (
        <div className="flex items-start justify-between gap-4 rounded-md border px-3 py-3">
          <div className="flex min-w-0 gap-3">
            <KeyRound className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
            <div className="min-w-0">
              <div className="text-sm font-medium">Import detected values as Citadel secrets</div>
              <div className="mt-1 text-xs text-muted-foreground">
                Encrypt {sensitiveAdoptionIssues.length} detected sensitive{' '}
                {sensitiveAdoptionIssues.length === 1 ? 'value' : 'values'} and bind{' '}
                {sensitiveAdoptionIssues.length === 1 ? 'it' : 'them'} to this deployment. The values never leave the
                server.
              </div>
            </div>
          </div>
          <Switch
            checked={importSensitiveEnvironmentAsSecrets}
            onCheckedChange={(checked) =>
              adoptFrom &&
              setSensitiveImportPreference({
                containerId: adoptFrom,
                enabled: checked,
              })
            }
            aria-label="Import detected values as Citadel secrets"
          />
        </div>
      )}
      {visibleAdoptionIssues.map((issue) => (
        <AlertMessage
          key={`${issue.code}:${issue.fieldPath ?? ''}`}
          type={issue.severity === AdoptionIssueSeverity.Blocker ? 'error' : 'warning'}
          title={issue.severity === AdoptionIssueSeverity.Blocker ? 'Adoption blocked' : 'Review required'}>
          {issue.message}
        </AlertMessage>
      ))}
      <FormShell
        mode={mode}
        schema={schema}
        original={original}
        update={update}
        setUpdate={setUpdate}
        onSave={handleSave}
        pending={isPending}
        disabled={disabled}
        saveDisabled={!!adoptFrom && (isAdoptionDraftLoading || !adoptionDraft || hasAdoptionBlocker)}
        saveLabel={adoptFrom ? 'Adopt Container' : 'Save'}
        confirmSave={adoptFrom ? confirmAdoption : undefined}
        draftKey={formDraftKey}
        draftVersion={1}
      />
      <Dialog
        open={adoptionConfirmationOpen}
        onOpenChange={(open) => {
          if (!open) {
            resetAdoptionConfirmation();
            resolveAdoptionConfirmation(false);
          }
        }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Adopt {adoptionDraft?.source.name ?? 'container'}?</DialogTitle>
            <DialogDescription>
              Citadel will start managing this existing container as a deployment. Docker will not be changed now.
              Future Apply operations will recreate it from the reviewed configuration.
              {importSensitiveEnvironmentAsSecrets &&
                ' Detected sensitive values will be stored as encrypted Citadel secrets.'}
            </DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-4 my-4">
            <p className="break-all">
              Please enter{' '}
              <button
                type="button"
                className="cursor-pointer font-bold"
                onClick={() => {
                  navigator.clipboard.writeText(adoptionContainerName);
                  toast(`Copied "${adoptionContainerName}" to clipboard!`);
                }}>
                {adoptionContainerName}
              </button>{' '}
              below to confirm this action.
              <br />
              <span className="text-xs text-muted-foreground">You may click the name in bold to copy it</span>
            </p>
            <Input
              aria-label={`Enter ${adoptionContainerName} to confirm`}
              value={adoptionConfirmationInput}
              onChange={(event) => setAdoptionConfirmationInput(event.target.value)}
              className="focus-visible:ring-1"
            />
          </div>
          <DialogFooter>
            <ConfirmButton
              ref={adoptionConfirmButtonRef}
              title="Adopt"
              icon={<PackagePlus className="h-4 w-4" />}
              disabled={adoptionConfirmationDisabled}
              onClick={handleAdoptionConfirmation}
              loading={adoptionConfirmationLoading}
            />
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};
