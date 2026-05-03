import { RoleView, RoleType, ResourceType } from '@/api/generated/api.types';
import { ContentCard } from '@/components/custom/content-card';
import { Badge } from '@/components/ui/badge';
import { MultiSelect } from '@/components/ui/multi-select';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { AlertRuleComponents } from '@/features/alerters/alert-rules';
import { DeploymentComponents } from '@/features/deployments';
import { GitRepoComponents } from '@/features/git-repos';
import { PlatformComponents } from '@/features/platforms';
import { RegistryComponents } from '@/features/registries';
import { useLocalStorage, useRead } from '@/lib/hooks';
import { RequiredFormComponents } from '@/pages/types';
import { Shield, Lock, Layers, KeyRound, Rss, User, Users } from 'lucide-react';

type PermissionMatrix = Record<string, string[]>;

const EMPTY_MATRIX: PermissionMatrix = {};
type ResourceIcon = React.ComponentType<{ className?: string }>;

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
  const permissionMatrix = useRead('getPermissionMatrix');
  const [selectedRoleId, setSelectedRoleId] = useLocalStorage<string | null>('access-selected-role-id', null);
  const selectedRole = roles.find((r) => r.id === selectedRoleId) ?? null;

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
                className={`w-full text-left px-4 py-3 flex items-center justify-between transition-colors border-l-4 ${
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
                ) : null}
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
    </div>
  );
};

const RoleDetail = ({ role, permissionMatrix }: { role: RoleView; permissionMatrix: PermissionMatrix }) => {
  const permissions = role.permissions ?? [];

  const getSelected = (resource: string) =>
    permissions.filter((p) => p.resourceType === resource).map((p) => p.resourceAction);

  const handleChange = (resource: string, values: string[]) => {
    // TODO: send update to backend
    console.log(resource, values);
  };
  const buildOptions = (actions: string[]) =>
    actions.map((a) => ({
      label: a,
      value: a,
    }));
  const orderedResources = (Object.keys(RESOURCE_ICONS) as ResourceType[]).filter((r) => permissionMatrix[r]);
  return (
    <div className="flex-1 overflow-auto p-4 space-y-6">
      <div>
        <div className="flex flex-row gap-2 items-center">
          <h2 className="text-base font-semibold">{role.name}</h2>
          <p className="flex flex-row gap-2 items-center text-xs text-amber-700 bg-amber-100 px-2 rounded-4xl">
            <Lock className="h-3 w-3" />
            {role.roleType === RoleType.System && 'System Role'}
          </p>
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
                          defaultValue={getSelected(resource)}
                          onValueChange={(vals) => handleChange(resource, vals)}
                          placeholder={`Select ${resource} permissions`}
                          maxCount={4}
                          resetOnDefaultValueChange={true}
                          animation={0}
                          disabled={role.roleType === RoleType.System}
                        />
                      </div>
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </ContentCard>
      </div>
    </div>
  );
};

export const RoleFormComponents: RequiredFormComponents = {
  AddForm: {
    Header: {
      title: 'Role',
    },
    Content: () => <></>,
  },
};
