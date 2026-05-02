import {
  PermissionView,
  ResourceAction,
  ResourceType as ApiResourceType,
  RoleView,
  RoleType,
} from '@/api/generated/api.types';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { RequiredFormComponents } from '@/pages/types';
import { Shield } from 'lucide-react';
import { useState } from 'react';

const ALL_RESOURCE_TYPES = Object.values(ApiResourceType);
const ALL_ACTIONS = Object.values(ResourceAction);

export const Roles = ({ items, isLoading }: { items: RoleView[]; isLoading: boolean }) => {
  const roles = items ?? [];
  const [selectedRoleId, setSelectedRoleId] = useState<string | null>(null);
  const selectedRole = roles.find((r) => r.id === selectedRoleId) ?? null;

  return (
    <div className="flex flex-col md:flex-row gap-4 min-h-[400px]">
      {/* Role list */}
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

      {/* Role detail */}
      <div className="flex-1 border border-border rounded-md overflow-hidden flex flex-col">
        {selectedRole ? (
          <RoleDetail role={selectedRole} />
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

const RoleDetail = ({ role }: { role: RoleView }) => {
  const permissions = role.permissions ?? [];

  const hasPermission = (resourceType: string, action: string): boolean =>
    permissions.some((p: PermissionView) => p.resourceType === resourceType && p.resourceAction === action);

  return (
    <>
      <div className="p-4 border-b border-border ">
        <h2 className="text-base font-semibold">{role.name}</h2>
        <p className="text-xs text-muted-foreground mt-0.5">
          {permissions.length} permission{permissions.length !== 1 ? 's' : ''}
        </p>
      </div>

      <div className="flex-1 overflow-auto p-4">
        <div className="flex flex-col gap-2">
          <p className="text-sm font-semibold">Permissions Matrix</p>
          <div className="border border-border border-t-0 rounded-sm overflow-hidden">
            <Table>
              <TableHeader className="bg-accent/60">
                <TableRow>
                  <TableHead className="w-[180px]">Resource Type</TableHead>
                  {ALL_ACTIONS.map((action) => (
                    <TableHead key={action} className="text-center">
                      {action}
                    </TableHead>
                  ))}
                </TableRow>
              </TableHeader>
              <TableBody>
                {ALL_RESOURCE_TYPES.map((rt) => (
                  <TableRow key={rt}>
                    <TableCell className="font-medium text-sm">{rt}</TableCell>
                    {ALL_ACTIONS.map((action) => (
                      <TableCell key={action} className="text-center">
                        <Checkbox checked={hasPermission(rt, action)} disabled />
                      </TableCell>
                    ))}
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </div>
      </div>
    </>
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
