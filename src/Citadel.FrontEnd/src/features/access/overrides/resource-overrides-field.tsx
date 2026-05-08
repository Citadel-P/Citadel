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
import { PluralResourceMap } from '@/api/types';
import { Link } from 'react-router';
import { Pencil, Plus, Trash2 } from 'lucide-react';
import { useMemo, useState, type ComponentType } from 'react';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

type ResourceItem = {
  id: string;
  name: string;
};

type OverrideResourceType =
  | ResourceType.Platform
  | ResourceType.Deployment
  | ResourceType.Stack
  | ResourceType.Registry
  | ResourceType.GitRepository;

type DisplayResourceItem = ResourceItem & {
  resourceType: OverrideResourceType;
};

type ResourceSelectionMap = Record<
  string,
  { permissionLevel: PermissionLevel | null; specificPermissions: SpecificPermission[] }
>;
type ResourceDraftSelections = Record<OverrideResourceType, ResourceSelectionMap>;

const OVERRIDE_RESOURCE_TYPES: OverrideResourceType[] = [
  ResourceType.Platform,
  ResourceType.Deployment,
  ResourceType.Stack,
  ResourceType.Registry,
  ResourceType.GitRepository,
];

const OVERRIDE_RESOURCE_ICONS: Record<OverrideResourceType, ComponentType<{ className?: string }>> = {
  [ResourceType.Platform]: CitadelIcons.Platform,
  [ResourceType.Deployment]: CitadelIcons.Deployment,
  [ResourceType.Stack]: CitadelIcons.Stack,
  [ResourceType.Registry]: CitadelIcons.Registry,
  [ResourceType.GitRepository]: CitadelIcons.GitRepository,
};

const isOverrideResourceType = (resourceType: ResourceType): resourceType is OverrideResourceType =>
  OVERRIDE_RESOURCE_TYPES.includes(resourceType as OverrideResourceType);

const createEmptyDraftSelections = (): ResourceDraftSelections =>
  Object.fromEntries(OVERRIDE_RESOURCE_TYPES.map((resourceType) => [resourceType, {}])) as ResourceDraftSelections;

const buildDraftSelections = (entries: Array<PermissionInput & { resourceId: string }>): ResourceDraftSelections => {
  const draft = createEmptyDraftSelections();

  for (const entry of entries) {
    if (!isOverrideResourceType(entry.resourceType)) continue;

    draft[entry.resourceType][entry.resourceId] = {
      permissionLevel: entry.permissionLevel,
      specificPermissions: entry.specificPermissions ?? [],
    };
  }

  return draft;
};

const flattenDraftSelections = (
  draftSelectionsByType: ResourceDraftSelections,
): Array<PermissionInput & { resourceId: string }> =>
  OVERRIDE_RESOURCE_TYPES.flatMap((resourceType) =>
    Object.entries(draftSelectionsByType[resourceType])
      .filter(([, state]) => state.permissionLevel)
      .map(([resourceId, state]) => ({
        resourceType,
        resourceId,
        permissionLevel: state.permissionLevel!,
        specificPermissions: state.specificPermissions.length > 0 ? state.specificPermissions : null,
      })),
  );

const getResourceRowKey = (resourceType: ResourceType, resourceId: string) => `${resourceType}:${resourceId}`;

const getResourceEditPath = (resourceType: OverrideResourceType, resourceId: string) =>
  `/${PluralResourceMap[resourceType].toLowerCase()}/edit/${resourceId}`;

const readResourceItems = (data: unknown): ResourceItem[] => {
  const raw = Object.values((data as Record<string, unknown>) ?? {}).at(0);
  return (Array.isArray(raw) ? raw : []) as ResourceItem[];
};

const useOverrideResourceItems = (resourceType: OverrideResourceType) => {
  const plural = PluralResourceMap[resourceType as keyof typeof PluralResourceMap];
  const read = useRead(`list${plural}` as any);
  const items = useMemo(() => readResourceItems(read.data?.data), [read.data]);

  return {
    items,
    isLoading: read.isLoading,
  };
};

const ResourceLinkCell = ({
  resourceType,
  resourceId,
  resourceName,
}: {
  resourceType: OverrideResourceType;
  resourceId: string;
  resourceName: string;
}) => {
  const Icon = OVERRIDE_RESOURCE_ICONS[resourceType];

  return (
    <TableCell>
      <Link
        to={getResourceEditPath(resourceType, resourceId)}
        className="table-link flex gap-2 items-center"
        title={resourceName}>
        <Icon className="h-3.5 w-3.5 text-muted-foreground" />
        {resourceName}
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
  value: Array<PermissionInput & { resourceId: string }> | null;
  onChange: (next: Array<PermissionInput & { resourceId: string }>) => void;
}) => {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [selectedType, setSelectedType] = useState<OverrideResourceType>(ResourceType.Platform);
  const [draftSelectionsByType, setDraftSelectionsByType] = useState<ResourceDraftSelections>(() =>
    createEmptyDraftSelections(),
  );

  const entries = useMemo(() => (value ?? []) as Array<PermissionInput & { resourceId: string }>, [value]);

  const { data: matrixData } = useRead('getPermissionMatrix');
  const permissionMatrix = useMemo(
    () => (matrixData?.data ?? {}) as Record<OverrideResourceType, PermissionMatrixViewItem>,
    [matrixData],
  );

  const resourceTypeOptions = useMemo(
    () =>
      OVERRIDE_RESOURCE_TYPES.filter((resourceType) => permissionMatrix[resourceType]).map((resourceType) => ({
        value: resourceType,
        label: resourceType,
        icon: OVERRIDE_RESOURCE_ICONS[resourceType],
      })),
    [permissionMatrix],
  );

  const effectiveSelectedType = useMemo<OverrideResourceType>(() => {
    const exists = resourceTypeOptions.some((option) => option.value === selectedType);
    if (exists) return selectedType;
    return (resourceTypeOptions[0]?.value as OverrideResourceType | undefined) ?? ResourceType.Platform;
  }, [resourceTypeOptions, selectedType]);

  const deploymentItems = useOverrideResourceItems(ResourceType.Deployment);
  const stackItems = useOverrideResourceItems(ResourceType.Stack);
  const platformItems = useOverrideResourceItems(ResourceType.Platform);
  const gitRepositoryItems = useOverrideResourceItems(ResourceType.GitRepository);
  const selectedTypeItems = useOverrideResourceItems(effectiveSelectedType);

  const resources = useMemo<DisplayResourceItem[]>(
    () =>
      selectedTypeItems.items.map((resource) => ({
        ...resource,
        resourceType: effectiveSelectedType,
      })),
    [selectedTypeItems.items, effectiveSelectedType],
  );

  const isResourcesLoading = selectedTypeItems.isLoading;

  const allResourceNameMap = useMemo(() => {
    const map = new Map<string, string>();

    const allKnownResources = [
      ...deploymentItems.items,
      ...stackItems.items,
      ...platformItems.items,
      ...gitRepositoryItems.items,
    ];

    for (const resource of allKnownResources) {
      map.set(resource.id, resource.name);
    }

    return map;
  }, [deploymentItems.items, stackItems.items, platformItems.items, gitRepositoryItems.items]);

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
        currentTypeSelections[resourceId] = { permissionLevel, specificPermissions };
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
                <TableHead className="flex-1">Specific</TableHead>
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
                      permissionLevel: entry.permissionLevel,
                      specificPermissions: entry.specificPermissions ?? [],
                    });
                  }
                  return grouped;
                }, new Map<string, { resourceType: ResourceType; resourceId: string; permissionLevel: PermissionLevel; specificPermissions: SpecificPermission[] }>()),
              )
                .map(([, row]) => row)
                .sort((a, b) => {
                  if (a.resourceType !== b.resourceType) {
                    return a.resourceType.localeCompare(b.resourceType);
                  }
                  const aName = allResourceNameMap.get(a.resourceId) ?? a.resourceId;
                  const bName = allResourceNameMap.get(b.resourceId) ?? b.resourceId;
                  return aName.localeCompare(bName);
                })
                .map((row) => {
                  const resourceType = row.resourceType as OverrideResourceType;
                  const resourceName = allResourceNameMap.get(row.resourceId) ?? row.resourceId;
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
                          <SelectTrigger className="w-45">
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
                          <div className="w-120">
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
        <DialogContent className="sm:max-w-225 max-h-[85vh] flex flex-col p-0">
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
                      <TableHead className="flex-1">Specific</TableHead>
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
