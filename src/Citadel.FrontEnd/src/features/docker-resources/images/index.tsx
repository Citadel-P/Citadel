import { HardDrive } from 'lucide-react';
import { ExternalRepositories } from './external-repositories';
import { LocalImagesTable } from './table';
import { RequiredDockerComponents, ResourceDataHookResult } from '@/pages/types';
import { useImagesGroup } from './hooks/useImagesGroup';
import { ResourceSelector } from '@/components/dsl/common';
import { ActionBar2 } from '@/components/custom/action-bar';
import { DeleteImageDropdown, DeleteImagesButtonGroup, InspectImageButtonGroup, InspectImageDropDown } from './actions';

export const ImageComponents: RequiredDockerComponents = {
  Icon: <HardDrive className="h-4 w-4" />,
  tabs: [
    {
      label: 'Local',
      Content: LocalImagesTable,
      useData: function (platformId: string): ResourceDataHookResult<any> {
        const { imagesInfo, isLoading } = useImagesGroup(platformId);
        return { items: imagesInfo?.images ?? [], isLoading };
      },
      header: {
        showAdd: false,
        showSearch: true,
      },
      ButtonActions: {},
      DropdownActions: {
        InspectImageDropDown,
        DeleteImageDropdown,
      },
      GroupActions: ({ items }) => {
        return <ActionBar2 type="Image" items={items} actions={[InspectImageButtonGroup, DeleteImagesButtonGroup]} />;
      },
    },
    {
      label: 'External',
      Content: ExternalRepositories,
      header: {
        showAdd: false,
        showSearch: false,
        Extra: () => {
          return <ResourceSelector type="Registry" disabled={false} align="start" placeholder="Select Registry" />;
        },
      },
    },
  ],
  filterItems: (items, search) => {
    if (!search.trim()) return items;
    const s = search.toLowerCase();
    return items.filter(
      (c) =>
        c.name?.toLowerCase().includes(s) ||
        c.dockerImageId?.toLowerCase().includes(s) ||
        c.dockerImageId?.substring(0, 12).toLowerCase().includes(s),
    );
  },
};
