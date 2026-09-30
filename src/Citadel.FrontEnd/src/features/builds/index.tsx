import { AuthorizedProject } from '@/api/generated/api.types';
import { ActionBar } from '@/components/custom/action-bar';
import { CitadelIcons } from '@/lib/icons';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { BuildDropdownActions, BuildGroupActions } from './actions';
import { useBuildsGroup } from './hooks/useBuildsGroup';
import { BuildsTable } from './table';

const EMPTY_BUILDS: never[] = [];

export const BuildComponents: RequiredComponents<AuthorizedProject> = {
  Icon: CitadelIcons.Build,
  Content: ({ items, actions, isLoading }) => <BuildsTable items={items} actions={actions} isLoading={isLoading} />,
  header: {
    title: 'Builds',
    subtitle: 'Build Docker images from Git repositories and push them to registries.',
    showSearch: true,
    showAdd: true,
    showTagFilter: true,
    addButtonTitle: 'Add Build',
  },
  DropdownActions: BuildDropdownActions,
  GroupActions: ({ items }) => <ActionBar type="Build" items={items} actions={Object.values(BuildGroupActions)} />,
  useData(): ResourceDataHookResult<AuthorizedProject> {
    const { projects, isLoading, capabilities, error, refetch, isFetching } = useBuildsGroup();
    return { error, refetch, isFetching, items: projects ?? EMPTY_BUILDS, isLoading, capabilities };
  },
  filterItems: filterBuilds,
};

function filterBuilds(items: AuthorizedProject[], search: string) {
  if (!search.trim()) return items;

  const value = search.toLowerCase();
  return items.filter(
    (project) =>
      project.name.toLowerCase().includes(value) ||
      (project.description ?? '').toLowerCase().includes(value) ||
      project.branch.toLowerCase().includes(value) ||
      project.imageRepository.toLowerCase().includes(value) ||
      project.tagTemplates.join(',').toLowerCase().includes(value),
  );
}
