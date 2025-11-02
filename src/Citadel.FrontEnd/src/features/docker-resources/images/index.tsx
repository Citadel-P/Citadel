import { HardDrive } from 'lucide-react';
import { ImagesTable } from './table';
import { RequiredDockerComponents, ResourceDataHookResult } from '@/pages/types';
import { useImagesGroup } from './hooks/useImagesGroup';
import { ActionBar2 } from '@/components/custom/action-bar';
import { DeleteImageDropdown, DeleteImagesButtonGroup, InspectImageButtonGroup, InspectImageDropDown } from './actions';
import PullImageForm, { PullButton } from './pull-image';

export const ImageComponents: RequiredDockerComponents = {
  Icon: <HardDrive className="h-4 w-4" />,

  Table: ImagesTable,
  useData: function (platformId: string): ResourceDataHookResult<any> {
    const { imagesInfo, isLoading } = useImagesGroup(platformId);
    return { items: imagesInfo?.images ?? [], isLoading };
  },
  header: {
    showAdd: false,
    showSearch: true,
    Extra: () => <PullButton />,
  },
  SubHeader: PullImageForm,
  DropdownActions: {
    InspectImageDropDown,
    DeleteImageDropdown,
  },
  GroupActions: ({ items }) => {
    return <ActionBar2 type="Image" items={items} actions={[InspectImageButtonGroup, DeleteImagesButtonGroup]} />;
  },

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
