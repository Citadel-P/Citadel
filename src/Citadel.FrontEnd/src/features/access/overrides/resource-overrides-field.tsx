import {
  ResourceType,
  PermissionLevel,
  SpecificPermission,
  PermissionInput,
  PermissionMatrixViewItem,
} from '@/api/generated/api.types';
import { MultiSelect } from '@/components/ui/multi-select';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { SelectField } from '@/components/custom/common';
import { ContentCard } from '@/components/custom/content-card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { useRead } from '@/lib/hooks';
import { CitadelIcons } from '@/lib/icons';
import { Link } from 'react-router';
import { Pencil, Plus, Trash2 } from 'lucide-react';
import { useMemo, useState, type ComponentType } from 'react';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import type { KnownResourceName } from '@/api/types';

type ResourceItem = {
  id: string;
  name: string;
};

type OverrideResourceType =
  | ResourceType.Platform
  | ResourceType.Deployment
  | ResourceType.Stack
  | ResourceType.Registry
  | ResourceType.GitRepository
  | ResourceType.BackupPolicy
  | ResourceType.BackupRepository
  | ResourceType.AutomationAction
  | ResourceType.Build
  | ResourceType.BuildAgentPool;

type DisplayResourceItem = ResourceItem & {
  resourceType: OverrideResourceType;
};

type ResourceAccessEntry = PermissionInput & {
  resourceId: string;
  resourceName?: string | null;
};

type ResourceSelectionMap = Record<
  string,
  { permissionLevel: PermissionLevel | null; specificPermissions: SpecificPermission[]; resourceName?: string | null }
>;
type ResourceDraftSelections = Record<OverrideResourceType, ResourceSelectionMap>;
type OverrideResourceConfig = {
  icon: ComponentType<{ className?: string }>;
  listQuery: KnownResourceName;
  readItems: (data: unknown) => ResourceItem[];
  getEditPath: (resourceId: string) => string;
};

const OVERRIDE_RESOURCE_TYPES: OverrideResourceType[] = [
  ResourceType.Platform,
  ResourceType.Deployment,
  ResourceType.Stack,
  ResourceType.Registry,
  ResourceType.GitRepository,
  ResourceType.BackupPolicy,
  ResourceType.BackupRepository,
  ResourceType.AutomationAction,
  ResourceType.Build,
  ResourceType.BuildAgentPool,
];

const readItemsFrom = (propertyName: string) => (data: unknown): ResourceItem[] => {
  const raw = (data as Record<string, unknown> | null | undefined)?.[propertyName];
  return (Array.isArray(raw) ? raw : []) as ResourceItem[];
};

const OVERRIDE_RESOURCE_CONFIG: Record<OverrideResourceType, OverrideResourceConfig> = {
  [ResourceType.Platform]: {
    icon: CitadelIcons.Platform,
    listQuery: 'listPlatforms',
    readItems: readItemsFrom('platforms'),
    getEditPath: (resourceId) => `/platforms/edit/${resourceId}`,
  },
  [ResourceType.Deployment]: {
    icon: CitadelIcons.Deployment,
    listQuery: 'listDeployments',
    readItems: readItemsFrom('deployments'),
    getEditPath: (resourceId) => `/deployments/edit/${resourceId}`,
  },
  [ResourceType.Stack]: {
    icon: CitadelIcons.Stack,
    listQuery: 'listStacks',
    readItems: readItemsFrom('stacks'),
    getEditPath: (resourceId) => `/stacks/edit/${resourceId}`,
  },
  [ResourceType.Registry]: {
    icon: CitadelIcons.Registry,
    listQuery: 'listRegistries',
    readItems: readItemsFrom('registries'),
    getEditPath: (resourceId) => `/registries/edit/${resourceId}`,
  },
  [ResourceType.GitRepository]: {
    icon: CitadelIcons.GitRepository,
    listQuery: 'listGitRepositories',
    readItems: readItemsFrom('gitRepositories'),
    getEditPath: (resourceId) => `/git-repos/edit/${resourceId}`,
  },
  [ResourceType.BackupRepository]: {
    icon: CitadelIcons.BackupRepository,
    listQuery: 'listBackupRepositories',
    readItems: readItemsFrom('repositories'),
    getEditPath: (resourceId) => `/backup-repositories/edit/${resourceId}`,
  },
  [ResourceType.BackupPolicy]: {
    icon: CitadelIcons.BackupPolicy,
    listQuery: 'listBackupPolicies',
    readItems: readItemsFrom('policies'),
    getEditPath: (resourceId) => `/backup-policies/edit/${resourceId}`,
  },
  [ResourceType.AutomationAction]: {
    icon: CitadelIcons.AutomationAction,
    listQuery: 'listAutomationActions',
    readItems: readItemsFrom('actions'),
    getEditPath: (resourceId) => `/automation/edit/${resourceId}`,
  },
  [ResourceType.Build]: {
    icon: CitadelIcons.Build,
    listQuery: 'listBuildProjects',
    readItems: readItemsFrom('projects'),
    getEditPath: (resourceId) => `/builds/edit/${resourceId}`,
  },
  [ResourceType.BuildAgentPool]: {
    icon: CitadelIcons.BuildAgentPool,
    listQuery: 'listBuildAgentPools',
    readItems: readItemsFrom('pools'),
    getEditPath: (resourceId) => `/build-pools/edit/${resourceId}`,
  },
};

const isOverrideResourceType = (resourceType: ResourceType): resourceType is OverrideResourceType =>
  OVERRIDE_RESOURCE_TYPES.includes(resourceType as OverrideResourceType);

const createEmptyDraftSelections = (): ResourceDraftSelections =>
  Object.fromEntries(OVERRIDE_RESOURCE_TYPES.map((resourceType) => [resourceType, {}])) as ResourceDraftSelections;

const buildDraftSelections = (entries: ResourceAccessEntry[]): ResourceDraftSelections => {
  const draft = createEmptyDraftSelections();

  for (const entry of entries) {
    if (!isOverrideResourceType(entry.resourceType)) continue;

    draft[entry.resourceType][entry.resourceId] = {
      permissionLevel: entry.permissionLevel,
      specificPermissions: entry.specificPermissions ?? [],
      resourceName: entry.resourceName,
    };
  }

  return draft;
};

const flattenDraftSelections = (draftSelectionsByType: ResourceDraftSelections): ResourceAccessEntry[] =>
  OVERRIDE_RESOURCE_TYPES.flatMap((resourceType) =>
    Object.entries(draftSelectionsByType[resourceType])
      .filter(([, state]) => state.permissionLevel)
      .map(([resourceId, state]) => ({
        resourceType,
        resourceId,
        resourceName: state.resourceName ?? null,
        permissionLevel: state.permissionLevel!,
        specificPermissions: state.specificPermissions.length > 0 ? state.specificPermissions : null,
      })),
  );

const getResourceRowKey = (resourceType: ResourceType, resourceId: string) => `${resourceType}:${resourceId}`;

const getResourceEditPath = (resourceType: OverrideResourceType, resourceId: string) =>
  OVERRIDE_RESOURCE_CONFIG[resourceType].getEditPath(resourceId);

const rowName = (resourceName: string | null | undefined, resourceId: string) => resourceName ?? resourceId;

const ResourceLinkCell = ({
  resourceType,
  resourceId,
  resourceName,
}: {
  resourceType: OverrideResourceType;
  resourceId: string;
  resourceName: string;
}) => {
  const Icon = OVERRIDE_RESOURCE_CONFIG[resourceType].icon;

  return (
    <TableCell>
      <Link
        to={getResourceEditPath(resourceType, resourceId)}
        className="table-link flex gap-2 items-center max-w-48 sm:max-w-72"
        title={resourceName}>
        <Icon className="h-3.5 w-3.5 text-muted-foreground" />
        <span className="truncate">{resourceName}</span>
      </Link>
    </TableCell>
  );
};

const ResourceActionsCell = ({ onEdit, onDelete }: { onEdit: () => void; onDelete: () => void }) => (
  <TableCell>
    <div className="flex items-center justify-end gap-1">
      <Button variant="ghost" size="icon" onClick={onEdit}>
        <Pencil className="size-4 text-muted-foreground" />
      </Button>
      <Button variant="ghost" size="icon" onClick={onDelete}>
        <Trash2 className="size-4 text-muted-foreground" />
      </Button>
    </div>
  </TableCell>
);

export const ResourceOverridesField = ({
  value,
  onChange,
}: {
  value: ResourceAccessEntry[] | null;
  onChange: (next: ResourceAccessEntry[]) => void;
}) => {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [selectedType, setSelectedType] = useState<OverrideResourceType>(ResourceType.Platform);
  const [draftSelectionsByType, setDraftSelectionsByType] = useState<ResourceDraftSelections>(() =>
    createEmptyDraftSelections(),
  );

  const entries = useMemo(() => (value ?? []) as ResourceAccessEntry[], [value]);

  const { data: matrixData } = useRead('getPermissionMatrix');
  const permissionMatrix = useMemo(
    () => (matrixData?.data ?? {}) as Record<OverrideResourceType, PermissionMatrixViewItem>,
    [matrixData],
  );

  const resourceTypeOptions = useMemo(
    () =>
      OVERRIDE_RESOURCE_TYPES.filter((resourceType) => permissionMatrix[resourceType]).map((resourceType) => ({
        value: resourceType,
        label: permissionMatrix[resourceType]?.label ?? resourceType,
        icon: OVERRIDE_RESOURCE_CONFIG[resourceType].icon,
      })),
    [permissionMatrix],
  );

  const effectiveSelectedType = useMemo<OverrideResourceType>(() => {
    const exists = resourceTypeOptions.some((option) => option.value === selectedType);
    if (exists) return selectedType;
    return (resourceTypeOptions[0]?.value as OverrideResourceType | undefined) ?? ResourceType.Platform;
  }, [resourceTypeOptions, selectedType]);

  const selectedTypeConfig = OVERRIDE_RESOURCE_CONFIG[effectiveSelectedType];
  const selectedTypeRead = useRead(selectedTypeConfig.listQuery as any, undefined, { enabled: dialogOpen });

  const resources = useMemo<DisplayResourceItem[]>(
    () =>
      selectedTypeConfig.readItems(selectedTypeRead.data?.data).map((resource) => ({
        ...resource,
        resourceType: effectiveSelectedType,
      })),
    [selectedTypeRead.data, effectiveSelectedType, selectedTypeConfig],
  );

  const isResourcesLoading = selectedTypeRead.isLoading;
  const resourceNameById = useMemo(
    () => new Map(resources.map((resource) => [resource.id, resource.name] as const)),
    [resources],
  );

  const selectedByResourceKey = draftSelectionsByType[effectiveSelectedType];

  const openDialogWithDraft = (resourceType: OverrideResourceType) => {
    setSelectedType(resourceType);
    setDraftSelectionsByType(buildDraftSelections(entries));
    setDialogOpen(true);
  };

  const handleResourcePermissionChange = (
    resourceId: string,
    permissionLevel: PermissionLevel | null,
    specificPermissions: SpecificPermission[],
  ) => {
    setDraftSelectionsByType((prev) => {
      const currentTypeSelections = { ...prev[effectiveSelectedType] };
      if (permissionLevel) {
        const existingName = currentTypeSelections[resourceId]?.resourceName;
        currentTypeSelections[resourceId] = {
          permissionLevel,
          specificPermissions,
          resourceName: existingName ?? resourceNameById.get(resourceId) ?? null,
        };
      } else {
        delete currentTypeSelections[resourceId];
      }

      return {
        ...prev,
        [effectiveSelectedType]: currentTypeSelections,
      };
    });
  };

  const handleEditRow = (resourceType: ResourceType) => {
    if (!isOverrideResourceType(resourceType)) return;
    openDialogWithDraft(resourceType);
  };

  return (
    <div className="space-y-3">
      <Button
        variant="outline"
        className="w-full max-w-100 flex flex-row gap-2"
        onClick={() => openDialogWithDraft(effectiveSelectedType)}>
        <Plus className="h-3 w-3" /> Grant Access
      </Button>

      {entries.length > 0 && (
        <ContentCard className="w-full">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Resource</TableHead>
                <TableHead className="w-32">Level</TableHead>
                <TableHead className="flex-1">Capabilities</TableHead>
                <TableHead className="text-center w-28">Action</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {Array.from(
                entries.reduce((grouped, entry) => {
                  const key = `${entry.resourceType}:${entry.resourceId}`;
                  const existing = grouped.get(key);
                  if (!existing) {
                    grouped.set(key, {
                      resourceType: entry.resourceType,
                      resourceId: entry.resourceId,
                      resourceName: entry.resourceName,
                      permissionLevel: entry.permissionLevel,
                      specificPermissions: entry.specificPermissions ?? [],
                    });
                  } else if (!existing.resourceName && entry.resourceName) {
                    existing.resourceName = entry.resourceName;
                  }
                  return grouped;
                }, new Map<string, { resourceType: ResourceType; resourceId: string; resourceName?: string | null; permissionLevel: PermissionLevel; specificPermissions: SpecificPermission[] }>()),
              )
                .map(([, row]) => row)
                .sort((a, b) => {
                  if (a.resourceType !== b.resourceType) {
                    return a.resourceType.localeCompare(b.resourceType);
                  }
                  const aName = rowName(a.resourceName, a.resourceId);
                  const bName = rowName(b.resourceName, b.resourceId);
                  return aName.localeCompare(bName);
                })
                .map((row) => {
                  const resourceType = row.resourceType as OverrideResourceType;
                  const resourceName = rowName(row.resourceName, row.resourceId);
                  const matrix = permissionMatrix[resourceType];
                  const availableSpecific =
                    row.permissionLevel && matrix
                      ? Object.entries(matrix.specificPermissions)
                          .filter(([, minLevel]) => {
                            const levels = [PermissionLevel.Read, PermissionLevel.Write, PermissionLevel.Execute];
                            return (
                              levels.indexOf(minLevel as PermissionLevel) <=
                              levels.indexOf(row.permissionLevel as PermissionLevel)
                            );
                          })
                          .map(([perm]) => perm)
                      : [];
                  return (
                    <TableRow key={getResourceRowKey(row.resourceType, row.resourceId)}>
                      <ResourceLinkCell
                        resourceType={resourceType}
                        resourceId={row.resourceId}
                        resourceName={resourceName}
                      />
                      <TableCell>
                        <Select value={row.permissionLevel ?? ''} disabled>
                          <SelectTrigger className="w-full max-w-full">
                            <SelectValue placeholder="Select level" />
                          </SelectTrigger>
                          <SelectContent className="bg-background">
                            {row.permissionLevel && (
                              <SelectItem value={row.permissionLevel}>{row.permissionLevel}</SelectItem>
                            )}
                          </SelectContent>
                        </Select>
                      </TableCell>
                      <TableCell>
                        {row.permissionLevel && availableSpecific.length > 0 ? (
                          <div className="w-full min-w-0 max-w-full">
                            <MultiSelect
                              options={availableSpecific.map((sp) => ({ label: sp, value: sp }))}
                              defaultValue={row.specificPermissions}
                              placeholder="Select capabilities"
                              onValueChange={() => {}}
                              maxCount={3}
                              animation={0}
                              disabled
                            />
                          </div>
                        ) : (
                          <p className="text-xs text-muted-foreground">
                            {row.permissionLevel ? 'No capabilities' : '—'}
                          </p>
                        )}
                      </TableCell>
                      <ResourceActionsCell
                        onEdit={() => handleEditRow(row.resourceType)}
                        onDelete={() =>
                          onChange(
                            entries.filter(
                              (entry) =>
                                !(entry.resourceType === row.resourceType && entry.resourceId === row.resourceId),
                            ),
                          )
                        }
                      />
                    </TableRow>
                  );
                })}
            </TableBody>
          </Table>
        </ContentCard>
      )}

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="sm:max-w-225 max-h-[85vh] flex flex-col p-0 ">
          <DialogHeader className="shrink-0 border-b px-6 py-4">
            <DialogTitle>Resource Overrides</DialogTitle>
            <DialogDescription>Select resources and assign direct permissions for this user.</DialogDescription>
          </DialogHeader>

          <div className="space-y-4 flex flex-col flex-1 overflow-hidden px-6 py-2">
            <div className="w-full sm:w-72 sm:ml-auto space-y-1">
              <SelectField
                value={effectiveSelectedType}
                onChange={(next) => setSelectedType(next as OverrideResourceType)}
                options={resourceTypeOptions}
                placeholder="Select resource type"
                allLabel="Resource Type"
                selectableLabel={false}
              />
            </div>

            <div className="overflow-auto flex-1 min-h-0">
              <ContentCard className="w-full overflow-auto max-h-[55vh]">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Resource</TableHead>
                      <TableHead className="w-32">Level</TableHead>
                      <TableHead className="flex-1">Capabilities</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {resources.length === 0 ? (
                      <TableRow>
                        <TableCell colSpan={2} className="text-sm text-muted-foreground">
                          {isResourcesLoading ? 'Loading resources...' : 'No resources found for this type.'}
                        </TableCell>
                      </TableRow>
                    ) : (
                      resources.map((resource) => {
                        const matrix = permissionMatrix[resource.resourceType];
                        if (!matrix) return null;
                        const levels = [PermissionLevel.Read, PermissionLevel.Write, PermissionLevel.Execute];
                        const maxIndex = levels.indexOf(matrix.maximumLevel as PermissionLevel);
                        const availableLevels = levels.filter((_, i) => i <= maxIndex);
                        const currentState = selectedByResourceKey[resource.id];
                        const currentLevel = currentState?.permissionLevel ?? null;
                        const currentSpecific = currentState?.specificPermissions ?? [];
                        const availableSpecific = !currentLevel
                          ? []
                          : Object.entries(matrix.specificPermissions)
                              .filter(
                                ([, minLevel]) =>
                                  levels.indexOf(minLevel as PermissionLevel) <= levels.indexOf(currentLevel),
                              )
                              .map(([perm]) => perm);

                        return (
                          <TableRow key={getResourceRowKey(resource.resourceType, resource.id)}>
                            <ResourceLinkCell
                              resourceType={resource.resourceType}
                              resourceId={resource.id}
                              resourceName={resource.name}
                            />
                            <TableCell>
                              <Select
                                value={currentLevel ?? ''}
                                onValueChange={(v) =>
                                  handleResourcePermissionChange(resource.id, v ? (v as PermissionLevel) : null, [])
                                }>
                                <SelectTrigger className="w-40">
                                  <SelectValue placeholder="Select level" />
                                </SelectTrigger>
                                <SelectContent className="bg-background">
                                  {availableLevels.map((level) => (
                                    <SelectItem key={level} value={level}>
                                      {level}
                                    </SelectItem>
                                  ))}
                                </SelectContent>
                              </Select>
                            </TableCell>
                            <TableCell>
                              {currentLevel && availableSpecific.length > 0 ? (
                                <MultiSelect
                                  options={availableSpecific.map((sp) => ({ label: sp, value: sp }))}
                                  defaultValue={currentSpecific}
                                  onValueChange={(v) =>
                                    handleResourcePermissionChange(resource.id, currentLevel, v as SpecificPermission[])
                                  }
                                  placeholder="Select capabilities"
                                  maxCount={3}
                                  animation={0}
                                  disabled={isResourcesLoading}
                                />
                              ) : (
                                <p className="text-xs text-muted-foreground">
                                  {currentLevel ? 'No capabilities' : '—'}
                                </p>
                              )}
                            </TableCell>
                          </TableRow>
                        );
                      })
                    )}
                  </TableBody>
                </Table>
              </ContentCard>
            </div>
          </div>

          <DialogFooter className="shrink-0 px-6 py-4">
            <Button variant="outline" onClick={() => setDialogOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={() => {
                onChange(flattenDraftSelections(draftSelectionsByType));
                setDialogOpen(false);
              }}>
              Save
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};
