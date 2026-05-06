import { TabbedResourceComponents } from '@/pages/types';
import { Users } from './users';
import { Teams } from './teams';
import { Roles, AddRoleButton } from './roles';
import { useUsersList } from './users/hooks/useUsersList';
import { ActionBar } from '@/components/custom/action-bar';
import { UserDropdownActions, UserGroupActions } from './users/actions';
import { useTeamsList } from './teams/hooks/useTeamsList';
import { TeamDropdownActions, TeamGroupActions } from './teams/actions';
import { useRead } from '@/lib/hooks';
import { CitadelIcons } from '@/lib/icons';

export const AccessComponents: TabbedResourceComponents = {
  Icon: CitadelIcons.Access,
  header: {
    subtitle: 'Manage users, teams, and their access.',
    showSearch: false,
    showAdd: false,
  },
  Tabs: [
    {
      label: 'Users',
      slug: 'users',
      Content: ({ items, isLoading, actions }) => {
        return <Users items={items} isLoading={isLoading} actions={actions} />;
      },
      Header: {
        showAdd: true,
        showSearch: false,
        addButtonTitle: 'Add User',
        addButtonUrl: '/access/users/add',
      },
      DropdownActions: UserDropdownActions,
      GroupActions: ({ items }) => {
        return <ActionBar type="User" items={items} actions={Object.values(UserGroupActions)} />;
      },
      useData: () => {
        const { pagedUsers, isLoading } = useUsersList();
        return {
          items: pagedUsers as any,
          isLoading,
        };
      },
    },
    {
      label: 'Teams',
      slug: 'teams',
      DropdownActions: TeamDropdownActions,
      GroupActions: ({ items }) => {
        return <ActionBar type="Team" items={items} actions={Object.values(TeamGroupActions)} />;
      },
      Content: ({ items, isLoading, actions }) => {
        return <Teams items={items} isLoading={isLoading} actions={actions} />;
      },
      Header: {
        showAdd: true,
        showSearch: false,
        addButtonTitle: 'Add Team',
        addButtonUrl: '/access/teams/add',
      },
      useData: () => {
        const { pagedUsers, isLoading } = useTeamsList();
        return {
          items: pagedUsers as any,
          isLoading,
        };
      },
    },
    {
      label: 'Roles',
      slug: 'roles',
      Content: ({ items, isLoading }) => {
        return <Roles items={items} isLoading={isLoading} />;
      },
      Header: {
        showSearch: false,
        Extra: AddRoleButton,
      },
      useData: () => {
        const { data, isLoading } = useRead('listRoles');
        return {
          items: data?.data.roles as any,
          isLoading,
        };
      },
    },
  ],
};
