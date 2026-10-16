import { useMemo } from 'react';
import { ImageUpdateStatus, type ImageView } from '@/api/generated/api.types';
import { UpdatesAvailableFilter, useUpdatesAvailableFilter } from '@/components/custom/updates-available-filter';
import { HardDrive } from 'lucide-react';
import { ImagesTable } from './table';
import { RegularResourceComponents, ResourceDataHookResult } from '@/pages/types';
import { useImagesGroup } from './hooks/useImagesGroup';
import { ActionBar } from '@/components/custom/action-bar';
import { ImageDropdownActions, ImageGroupActions } from './actions';
import PullImageForm, { PullButton } from './pull-image';
import { ImageColumnsButton } from './table-columns';

const EMPTY_IMAGES: never[] = [];

export const ImageComponents: RegularResourceComponents<ImageView> = {
  Icon: HardDrive,
  Content: ImagesTable,
  useData: function (platformId: string): ResourceDataHookResult<ImageView> {
    const { imagesInfo, capabilities, isLoading, error, refetch, isFetching } = useImagesGroup(platformId);
    const { updatesAvailableOnly } = useUpdatesAvailableFilter();
    const images = imagesInfo?.images ?? EMPTY_IMAGES;
    const items = useMemo(
      () =>
        updatesAvailableOnly
          ? images.filter(
              (image) =>
                !image.isStale &&
                image.updates?.some((update) => update.status === ImageUpdateStatus.UpdateAvailable && !update.isStale),
            )
          : images,
      [images, updatesAvailableOnly],
    );
    return { error, refetch, isFetching, items, isLoading, capabilities };
  },
  header: {
    showAdd: false,
    showSearch: true,
    activeFilterParams: ['updates'],
    Extra: () => (
      <>
        <ImageColumnsButton />
        <UpdatesAvailableFilter />
        <PullButton />
      </>
    ),
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
