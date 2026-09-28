import { HardDrive } from 'lucide-react';
import { ImagesTable } from './table';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { useImagesGroup } from './hooks/useImagesGroup';
import { ActionBar } from '@/components/custom/action-bar';
import { ImageDropdownActions, ImageGroupActions } from './actions';
import PullImageForm, { PullButton } from './pull-image';

const EMPTY_IMAGES: never[] = [];

export const ImageComponents: RequiredComponents = {
  Icon: HardDrive,
  Content: ImagesTable,
  useData: function (platformId: string): ResourceDataHookResult<any> {
    const { imagesInfo, capabilities, isLoading, error, refetch, isFetching } = useImagesGroup(platformId);
    return { error, refetch, isFetching, items: imagesInfo?.images ?? EMPTY_IMAGES, isLoading, capabilities };
  },
  header: {
    showAdd: false,
    showSearch: true,
    Extra: () => <PullButton />,
  },
  SubHeader: PullImageForm,
  DropdownActions: ImageDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Image" items={items} actions={Object.values(ImageGroupActions)} />;
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
