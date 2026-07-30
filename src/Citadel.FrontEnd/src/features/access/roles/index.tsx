import {
  RoleView,
  RoleType,
  ResourceType,
  PermissionLevel,
  SpecificPermission,
  PermissionInput,
  PermissionView,
  PermissionMatrixViewItem,
  LicenseCapability,
} from '@/api/generated/api.types';
import { ActionWithDialog } from '@/components/custom/action-with-dialog';
import { ContentCard } from '@/components/custom/content-card';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { MultiSelect } from '@/components/ui/multi-select';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { useLocalStorage, useMutate, useRead } from '@/lib/hooks';
import { Lock, Plus, Loader2, Trash } from 'lucide-react';
import { useState, useMemo } from 'react';
import { atom, useAtom } from 'jotai';
import { useQueryClient } from '@tanstack/react-query';
import { CitadelIcons } from '@/lib/icons';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import { LicensedFeatureDescription } from '@/components/custom/license-feature-indicator';

type PermissionMatrix = Record<string, PermissionMatrixViewItem>;

type ResourcePermissionState = {
  permissionLevel: PermissionLevel | null;
  specificPermissions: SpecificPermission[];
};

type SelectedPermissions = Record<string, ResourcePermissionState>;

const EMPTY_MATRIX: PermissionMatrix = {};
const PERMISSION_LEVELS: PermissionLevel[] = [PermissionLevel.Read, PermissionLevel.Write, PermissionLevel.Execute];

type ResourceIcon = React.ComponentType<{ className?: string }>;

const createRoleOpenAtom = atom(false);

const ROLE_PERMISSION_RESOURCES = [
  ResourceType.Platform,
  ResourceType.Deployment,
  ResourceType.Stack,
  ResourceType.Registry,
  ResourceType.GitRepository,
  ResourceType.GitAccount,
  ResourceType.Alert,
  ResourceType.AlertChannel,
  ResourceType.Binding,
  ResourceType.Tag,
  ResourceType.BackupPolicy,
  ResourceType.BackupRepository,
  ResourceType.AutomationAction,
  ResourceType.Build,
  ResourceType.BuildAgentPool,
] as const;

type RolePermissionResourceType = (typeof ROLE_PERMISSION_RESOURCES)[number];

export const AddRoleButton = () => {
  const [, setOpen] = useAtom(createRoleOpenAtom);
  const { hasCapability } = useLicenseEntitlements();
  const canCreateCustomRoles = hasCapability(LicenseCapability.CustomAccessControl);
  const button = (
    <Button onClick={() => setOpen(true)} className="bg-primary hover:bg-primary/80 text-sm px-2.5 py-2.5">
      <Plus className="h-3 w-3" /> Add Role
    </Button>
  );

  if (canCreateCustomRoles) return button;

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span data-testid="custom-role-license">
          <Button disabled className="bg-primary text-sm px-2.5 py-2.5">
            <Plus className="h-3 w-3" /> Add Role
          </Button>
        </span>
      </TooltipTrigger>
      <TooltipContent className="w-72">
        <LicensedFeatureDescription
          requiredLicense="Team"
          descriptionClassName="text-xs leading-4 text-primary-foreground"
          indicatorClassName="border-primary-foreground/30 bg-primary-foreground/10 text-primary-foreground dark:text-primary-foreground">
          Create custom roles and permission sets.
        </LicensedFeatureDescription>
      </TooltipContent>
    </Tooltip>
  );
};

export const RESOURCE_ICONS: Record<RolePermissionResourceType, ResourceIcon> = {
  [ResourceType.Platform]: CitadelIcons.Platform,
  [ResourceType.Deployment]: CitadelIcons.Deployment,
  [ResourceType.Stack]: CitadelIcons.Stack,
  [ResourceType.Registry]: CitadelIcons.Registry,
  [ResourceType.GitRepository]: CitadelIcons.GitRepository,
  [ResourceType.GitAccount]: CitadelIcons.GitAccount,
  [ResourceType.Alert]: CitadelIcons.AlertRule,
  [ResourceType.AlertChannel]: CitadelIcons.AlertChannel,
  [ResourceType.Binding]: CitadelIcons.Binding,
  [ResourceType.Tag]: CitadelIcons.Tag,
  [ResourceType.BackupPolicy]: CitadelIcons.BackupPolicy,
  [ResourceType.BackupRepository]: CitadelIcons.BackupRepository,
  [ResourceType.AutomationAction]: CitadelIcons.AutomationAction,
  [ResourceType.Build]: CitadelIcons.Build,
  [ResourceType.BuildAgentPool]: CitadelIcons.BuildAgentPool,
};

const buildPermissionsFromView = (permissions: PermissionView[]): SelectedPermissions => {
  const result: SelectedPermissions = {};
  for (const perm of permissions) {
    result[perm.resourceType] = {
      permissionLevel: perm.permissionLevel,
      specificPermissions: perm.specificPermissions ?? [],
    };
  }
  return result;
};

const buildPermissionInputs = (selected: SelectedPermissions): PermissionInput[] =>
  Object.entries(selected)
    .filter(([, state]) => state.permissionLevel)
    .map(([resourceType, state]) => ({
      resourceType: resourceType as ResourceType,
      permissionLevel: state.permissionLevel!,
      specificPermissions: state.specificPermissions.length > 0 ? state.specificPermissions : null,
    }));

const updatePermissionState = (
  selected: SelectedPermissions,
  resource: string,
  permissionLevel: PermissionLevel | null,
  specificPermissions: SpecificPermission[],
): SelectedPermissions => {
  const nextSelected = { ...selected };
  if (permissionLevel) {
    nextSelected[resource] = { permissionLevel, specificPermissions };
  } else {
    delete nextSelected[resource];
  }
  return nextSelected;
};

export const Roles = ({ items, isLoading }: { items: RoleView[]; isLoading: boolean }) => {
  const roles = items ?? [];
  const queryClient = useQueryClient();
  const permissionMatrix = useRead('getPermissionMatrix');
  const { mutate: deleteRoles, isPending: isDeletingRole } = useMutate('deleteRoles', {
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['listRoles'] });
    },
  });
  const [isCreateOpen, setCreateOpen] = useAtom(createRoleOpenAtom);

  const [selectedRoleId, setSelectedRoleId] = useLocalStorage<string | null>('access-selected-role-id', null);
  const selectedRole = roles.find((r) => r.id === selectedRoleId) ?? null;

  const handleDeleteRole = (role: RoleView) => {
    if (selectedRoleId === role.id) {
      setSelectedRoleId(null);
    }
    deleteRoles({ ids: [role.id] });
  };

  return (
    <div className="flex flex-col md:flex-row gap-4 min-h-100">
      <div className="w-full md:w-1/3 flex flex-col border border-border rounded-md overflow-hidden">
        <div className="p-3 border-b border-border bg-accent/60 font-medium text-sm">Roles</div>
        <div className="flex-1 overflow-auto divide-y divide-border">
          {isLoading ? (
            <div className="p-4 text-center text-muted-foreground text-sm">Loading…</div>
          ) : roles.length === 0 ? (
            <div className="p-4 text-center text-muted-foreground text-sm">No roles found</div>
          ) : (
            roles.map((role) => (
              <div
                key={role.id}
                role="button"
                tabIndex={0}
                onClick={() => setSelectedRoleId(role.id)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    setSelectedRoleId(role.id);
                  }
                }}
                className={`group w-full text-left px-4 py-3 flex items-center justify-between transition-colors border-l-4 cursor-pointer ${
                  selectedRoleId === role.id
                    ? 'bg-primary/5 border-l-primary'
                    : 'hover:bg-accent/40 border-l-transparent'
                }`}>
                <div className="flex items-center gap-3">
                  <CitadelIcons.Role className="w-4 h-4 text-muted-foreground" />
                  <span className="text-sm font-medium">{role.name}</span>
                </div>
                {role.roleType === RoleType.System ? (
                  <Badge variant="outline" className="text-xs bg-accent/60">
                    System
                  </Badge>
                ) : (
                  <ActionWithDialog
                    name={role.name}
                    title="Delete"
                    icon={<Trash className="h-4 w-4" />}
                    iconPosition="left"
                    onClick={() => handleDeleteRole(role)}
                    disabled={isDeletingRole}
                    variant="destructive"
                    targetClassName="!h-7 !w-7 !min-w-7 !max-w-7 !flex-none !p-0 justify-center gap-0 opacity-0 group-hover:opacity-100 [&>span]:hidden"
                  />
                )}
              </div>
            ))
          )}
        </div>
      </div>

      <div className="flex-1 border border-border rounded-md overflow-hidden flex flex-col">
        {selectedRole ? (
          <RoleDetail role={selectedRole} permissionMatrix={permissionMatrix.data?.data ?? EMPTY_MATRIX} />
        ) : (
          <div className="flex-1 flex items-center justify-center flex-col gap-3 text-muted-foreground p-8">
            <CitadelIcons.Role className="w-10 h-10 text-muted-foreground/30" />
            <p className="text-sm">Select a role to view its permissions</p>
          </div>
        )}
      </div>

      <CreateRoleDialog
        open={isCreateOpen}
        onOpenChange={setCreateOpen}
        permissionMatrix={permissionMatrix.data?.data ?? EMPTY_MATRIX}
      />
    </div>
  );
};

const RoleDetail = ({ role, permissionMatrix }: { role: RoleView; permissionMatrix: PermissionMatrix }) => {
  const queryClient = useQueryClient();
  const { hasCapability } = useLicenseEntitlements();
  const canExpandPermissions = hasCapability(LicenseCapability.CustomAccessControl);
  const { mutate: updatePermissions } = useMutate('updateRolePermissions', {
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['listRoles'] });
    },
  });
  const permissions = useMemo(() => (role.permissions ?? []) as PermissionView[], [role.permissions]);
  const selectedPermissions = useMemo(() => buildPermissionsFromView(permissions), [permissions]);

  const handleChange = (
    resource: string,
    permissionLevel: PermissionLevel | null,
    specificPermissions: SpecificPermission[],
  ) => {
    const nextSelected = updatePermissionState(selectedPermissions, resource, permissionLevel, specificPermissions);
    updatePermissions({ id: role.id, data: { permissions: buildPermissionInputs(nextSelected) } });
  };

  return (
    <div className="flex-1 overflow-auto p-4 space-y-6">
      <div>
        <div className="flex flex-row gap-2 items-center">
          <h2 className="text-base font-semibold">{role.name}</h2>
          {role.roleType === RoleType.System && (
            <p className="flex flex-row gap-2 items-center text-xs text-amber-700 bg-amber-100 px-2 rounded-4xl">
              <Lock className="h-3 w-3" />
              System Role
            </p>
          )}
        </div>
        <p className="text-xs text-muted-foreground">
          {role.roleType === RoleType.System ? (
            'System roles are read-only and cannot be modified.'
          ) : (
            <>
              {permissions.length} permission{permissions.length !== 1 && 's'}
              {!canExpandPermissions ? ' - Team is required to expand permissions' : null}
            </>
          )}
        </p>
      </div>
      <div className="-mx-4 border-t border-border" />

      <div>
        <h6 className="font-medium mb-4">Permissions Matrix</h6>
        <PermissionsMatrixTable
          permissionMatrix={permissionMatrix}
          selectedPermissions={selectedPermissions}
          onPermissionChange={handleChange}
          disabled={role.roleType === RoleType.System}
          allowExpansion={canExpandPermissions}
        />
      </div>
    </div>
  );
};

const CreateRoleDialog = ({
  open,
  onOpenChange,
  permissionMatrix,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  permissionMatrix: PermissionMatrix;
}) => {
  const [roleName, setRoleName] = useState('');
  const queryClient = useQueryClient();
  const { mutate, isPending } = useMutate('createRole', {
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['listRoles'] });
      onOpenChange(false);
      setRoleName('');
      setSelectedPermissions({});
    },
  });
  const [selectedPermissions, setSelectedPermissions] = useState<SelectedPermissions>({});

  const handlePermissionChange = (
    resource: string,
    permissionLevel: PermissionLevel | null,
    specificPermissions: SpecificPermission[],
  ) => {
    setSelectedPermissions((prev) => updatePermissionState(prev, resource, permissionLevel, specificPermissions));
  };

  const handleCreate = () => {
    const permissions = buildPermissionInputs(selectedPermissions);
    mutate({ name: roleName, permissions });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-225 max-h-[85vh] flex flex-col">
        <DialogHeader className="shrink-0">
          <DialogTitle>Add Role</DialogTitle>
          <DialogDescription>Enter a role name and assign permissions by resource.</DialogDescription>
        </DialogHeader>

        <div className="space-y-4 flex flex-col flex-1 overflow-hidden px-1">
          <div className="space-y-2 shrink-0">
            <label htmlFor="create-role-name" className="text-sm font-medium">
              Role name
            </label>
            <Input
              id="create-role-name"
              value={roleName}
              onChange={(e) => setRoleName(e.target.value)}
              placeholder="e.g. Deployment Operator"
            />
          </div>

          <div className="flex flex-col flex-1 overflow-hidden">
            <h6 className="font-medium mb-3 shrink-0">Permissions Matrix</h6>
            <div className="overflow-y-auto flex-1">
              <PermissionsMatrixTable
                permissionMatrix={permissionMatrix}
                selectedPermissions={selectedPermissions}
                onPermissionChange={handlePermissionChange}
              />
            </div>
          </div>
        </div>

        <DialogFooter className="shrink-0">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleCreate} disabled={!roleName.trim() || isPending}>
            Create Role {isPending && <Loader2 className="w-4 h-4 animate-spin mr-1" />}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

const PermissionsMatrixTable = ({
  permissionMatrix,
  selectedPermissions,
  onPermissionChange,
  disabled,
  allowExpansion = true,
}: {
  permissionMatrix: PermissionMatrix;
  selectedPermissions: SelectedPermissions;
  onPermissionChange: (
    resource: string,
    permissionLevel: PermissionLevel | null,
    specificPermissions: SpecificPermission[],
  ) => void;
  disabled?: boolean;
  allowExpansion?: boolean;
}) => {
  const orderedResources = ROLE_PERMISSION_RESOURCES.filter((resource) => permissionMatrix[resource]);

  return (
    <ContentCard>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-48">Resource</TableHead>
            <TableHead className="w-32">Level</TableHead>
            <TableHead className="flex-1">Capabilities</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {orderedResources.map((resource) => {
            const matrixData = permissionMatrix[resource];
            const Icon = RESOURCE_ICONS[resource];
            const currentState = selectedPermissions[resource];

            return (
              <PermissionMatrixRow
                key={resource}
                resource={resource}
                icon={Icon}
                matrixData={matrixData}
                currentState={currentState}
                onPermissionChange={onPermissionChange}
                disabled={disabled}
                allowExpansion={allowExpansion}
              />
            );
          })}
        </TableBody>
      </Table>
    </ContentCard>
  );
};

const PermissionMatrixRow = ({
  resource,
  icon: Icon,
  matrixData,
  currentState,
  onPermissionChange,
  disabled,
  allowExpansion,
}: {
  resource: ResourceType;
  icon: ResourceIcon;
  matrixData: PermissionMatrixViewItem;
  currentState?: ResourcePermissionState;
  onPermissionChange: (
    resource: string,
    permissionLevel: PermissionLevel | null,
    specificPermissions: SpecificPermission[],
  ) => void;
  disabled?: boolean;
  allowExpansion: boolean;
}) => {
  const currentLevel = currentState?.permissionLevel ?? null;
  const currentSpecific = useMemo(() => currentState?.specificPermissions ?? [], [currentState?.specificPermissions]);

  const maxLevelIndex = useMemo(
    () => PERMISSION_LEVELS.indexOf(matrixData.maximumLevel as PermissionLevel),
    [matrixData.maximumLevel],
  );
  const availableLevels = useMemo(() => {
    const currentLevelIndex = currentLevel ? PERMISSION_LEVELS.indexOf(currentLevel) : -1;
    return PERMISSION_LEVELS.filter((_, i) => i <= maxLevelIndex && (allowExpansion || i <= currentLevelIndex));
  }, [allowExpansion, currentLevel, maxLevelIndex]);

  const availableSpecific = useMemo(() => {
    if (!currentLevel) return [];
    const currentLevelIndex = PERMISSION_LEVELS.indexOf(currentLevel);
    return Object.entries(matrixData.specificPermissions)
      .filter(([, minLevel]) => PERMISSION_LEVELS.indexOf(minLevel as PermissionLevel) <= currentLevelIndex)
      .map(([perm]) => perm as SpecificPermission)
      .filter((permission) => allowExpansion || currentSpecific.includes(permission));
  }, [allowExpansion, currentLevel, currentSpecific, matrixData.specificPermissions]);

  const handlePermissionChange = (level?: string, values?: string[]) => {
    if (level !== undefined) {
      const permLevel = level && level !== 'NONE' ? (level as PermissionLevel) : null;
      const levelIndex = permLevel ? PERMISSION_LEVELS.indexOf(permLevel) : -1;
      const filtered = permLevel
        ? currentSpecific.filter(
            (s) => PERMISSION_LEVELS.indexOf(matrixData.specificPermissions[s] as PermissionLevel) <= levelIndex,
          )
        : [];
      onPermissionChange(resource, permLevel, filtered);
    } else if (values !== undefined) {
      onPermissionChange(resource, currentLevel, values as SpecificPermission[]);
    }
  };

  return (
    <TableRow>
      <TableCell className="font-normal text-sm">
        <div className="flex items-center gap-3">
          <Icon className="h-3.5 w-3.5 text-muted-foreground" />
          <span>{matrixData.label ?? resource}</span>
        </div>
      </TableCell>
      <TableCell>
        <Select
          value={currentLevel ?? 'NONE'}
          onValueChange={(v) => handlePermissionChange(v)}
          disabled={disabled || (!allowExpansion && !currentLevel)}>
          <SelectTrigger className={currentLevel === null ? 'w-full text-muted-foreground/90' : 'w-full'}>
            <SelectValue placeholder="Select level" />
          </SelectTrigger>
          <SelectContent className="text-xs bg-background">
            <SelectItem value="NONE">None</SelectItem>
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
            options={availableSpecific.map((sp) => ({
              label: matrixData.specificPermissionLabels?.[sp] ?? sp,
              value: sp,
            }))}
            defaultValue={currentSpecific}
            onValueChange={(v) => handlePermissionChange(undefined, v)}
            placeholder="Select capabilities"
            maxCount={4}
            animation={0}
            disabled={disabled}
          />
        ) : (
          <p className="text-xs text-muted-foreground">{currentLevel ? 'No capabilities' : '—'}</p>
        )}
      </TableCell>
    </TableRow>
  );
};
