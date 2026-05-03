import { RoleView, RoleType, ResourceType, ResourceAction, PermissionInput } from '@/api/generated/api.types';
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
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { AlertRuleComponents } from '@/features/alerters/alert-rules';
import { DeploymentComponents } from '@/features/deployments';
import { GitRepoComponents } from '@/features/git-repos';
import { PlatformComponents } from '@/features/platforms';
import { RegistryComponents } from '@/features/registries';
import { useLocalStorage, useMutate, useRead } from '@/lib/hooks';
import { Shield, Lock, Layers, KeyRound, Rss, User, Users, Plus, Loader2, Trash } from 'lucide-react';
import { useEffect, useState } from 'react';
import { atom, useAtom } from 'jotai';
import { useQueryClient } from '@tanstack/react-query';

type PermissionMatrix = Record<string, string[]>;

const EMPTY_MATRIX: PermissionMatrix = {};
type ResourceIcon = React.ComponentType<{ className?: string }>;

const createRoleOpenAtom = atom(false);

export const AddRoleButton = () => {
  const [, setOpen] = useAtom(createRoleOpenAtom);
  return (
    <Button onClick={() => setOpen(true)} className="bg-primary hover:bg-primary/80 text-sm px-2.5 py-2.5">
      <Plus className="h-3 w-3" /> Add Role
    </Button>
  );
};

export const RESOURCE_ICONS: Record<ResourceType, ResourceIcon> = {
  Platform: PlatformComponents.Icon ?? Shield,
  Deployment: DeploymentComponents.Icon ?? Layers,
  Stack: Layers,

  Registry: RegistryComponents.Icon ?? Rss,
  GitRepository: GitRepoComponents.Icon ?? KeyRound,
  GitAccount: KeyRound,

  Alert: AlertRuleComponents.Icon ?? Rss,
  AlertChannel: Rss,

  User: User,
  Team: Users,
  Role: Shield,
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
    <div className="flex flex-col md:flex-row gap-4 min-h-[400px]">
      <div className="w-full md:w-1/3 flex flex-col border border-border rounded-md overflow-hidden">
        <div className="p-3 border-b border-border bg-accent/60 font-medium text-sm">Roles</div>
        <div className="flex-1 overflow-auto divide-y divide-border">
          {isLoading ? (
            <div className="p-4 text-center text-muted-foreground text-sm">Loading…</div>
          ) : roles.length === 0 ? (
            <div className="p-4 text-center text-muted-foreground text-sm">No roles found</div>
          ) : (
            roles.map((role) => (
              <button
                key={role.id}
                onClick={() => setSelectedRoleId(role.id)}
                className={`group w-full text-left px-4 py-3 flex items-center justify-between transition-colors border-l-4 ${
                  selectedRoleId === role.id
                    ? 'bg-primary/5 border-l-primary'
                    : 'hover:bg-accent/40 border-l-transparent'
                }`}>
                <div className="flex items-center gap-3">
                  <Shield className="w-4 h-4 text-muted-foreground" />
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
              </button>
            ))
          )}
        </div>
      </div>

      <div className="flex-1 border border-border rounded-md overflow-hidden flex flex-col">
        {selectedRole ? (
          <RoleDetail role={selectedRole} permissionMatrix={permissionMatrix.data?.data ?? EMPTY_MATRIX} />
        ) : (
          <div className="flex-1 flex items-center justify-center flex-col gap-3 text-muted-foreground p-8">
            <Shield className="w-10 h-10 text-muted-foreground/30" />
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
  const { mutate: updatePermissions } = useMutate('updateRolePermissions', {
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['listRoles'] });
    },
  });
  const permissions = role.permissions ?? [];

  const selectedByResource = permissions.reduce<Record<string, string[]>>((acc, p) => {
    const key = String(p.resourceType);
    if (!acc[key]) acc[key] = [];
    acc[key].push(String(p.resourceAction));
    return acc;
  }, {});

  const handleChange = (resource: string, values: string[]) => {
    const nextSelectedByResource = {
      ...selectedByResource,
      [resource]: values,
    };

    const nextPermissions: PermissionInput[] = Object.entries(nextSelectedByResource).flatMap(
      ([resourceType, resourceActions]) =>
        resourceActions.map((resourceAction) => ({
          resourceType: resourceType as ResourceType,
          resourceAction: resourceAction as ResourceAction,
        })),
    );

    updatePermissions({
      id: role.id,
      data: { permissions: nextPermissions },
    });
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
            </>
          )}
        </p>
      </div>
      <div className="-mx-4 border-t border-border" />

      <div>
        <h6 className="font-medium mb-4">Permissions Matrix</h6>
        <PermissionsMatrixTable
          permissionMatrix={permissionMatrix}
          selectedByResource={selectedByResource}
          onResourceChange={handleChange}
          disabled={role.roleType === RoleType.System}
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
    },
  });
  const [selectedByResource, setSelectedByResource] = useState<Record<string, string[]>>({});

  useEffect(() => {
    if (!open) return;
    setRoleName('');
    setSelectedByResource({});
  }, [open]);

  const handleResourceChange = (resource: string, values: string[]) => {
    setSelectedByResource((prev) => ({ ...prev, [resource]: values }));
  };

  const handleCreate = () => {
    const permissions: PermissionInput[] = Object.entries(selectedByResource).flatMap(([resourceType, actions]) =>
      actions.map((resourceAction) => ({
        resourceType: resourceType as ResourceType,
        resourceAction: resourceAction as ResourceAction,
      })),
    );

    mutate({ name: roleName, permissions });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[900px] max-h-[85vh] flex flex-col">
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
                selectedByResource={selectedByResource}
                onResourceChange={handleResourceChange}
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
  selectedByResource,
  onResourceChange,
  disabled,
}: {
  permissionMatrix: PermissionMatrix;
  selectedByResource: Record<string, string[]>;
  onResourceChange: (resource: string, values: string[]) => void;
  disabled?: boolean;
}) => {
  const buildOptions = (actions: string[]) =>
    actions.map((a) => ({
      label: a,
      value: a,
    }));

  const orderedResources = (Object.keys(RESOURCE_ICONS) as ResourceType[]).filter((r) => permissionMatrix[r]);

  return (
    <ContentCard>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-[220px]">Resource</TableHead>
            <TableHead>Permissions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {orderedResources.map((resource) => {
            const actions = permissionMatrix[resource];
            const Icon = RESOURCE_ICONS[resource];

            return (
              <TableRow key={resource}>
                <TableCell className="font-normal text-sm">
                  <div className="flex items-center gap-3">
                    <Icon className="h-3.5 w-3.5 text-muted-foreground" />
                    <span>{resource}</span>
                  </div>
                </TableCell>
                <TableCell>
                  <div className="max-w-xl">
                    <MultiSelect
                      options={buildOptions(actions)}
                      defaultValue={selectedByResource[resource] ?? []}
                      onValueChange={(vals) => onResourceChange(resource, vals)}
                      placeholder={`Select ${resource} permissions`}
                      maxCount={4}
                      resetOnDefaultValueChange={true}
                      animation={0}
                      disabled={disabled}
                    />
                  </div>
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </ContentCard>
  );
};
