import { useCallback, useEffect, useState } from 'react';
import { Package, Award, Star } from 'lucide-react';
import { DockerHubImageResult } from '@/api/generated/api.types';
import { PullImageBadge } from '@/components/ui/PullImageBadge';
import Loader from '@/components/ui/loader';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { formatNumber } from '@/lib/utils';
import { SearchField } from '@/components/ui/SearchField';
import { useDebounce } from '@/hooks/useDebounce';
import { useRead } from '@/lib/hooks';
import { useResourceFilter, useTaskSheet } from '@/lib/atoms';

export function PublicDockerHubImages() {
  const [searchValue, setSearchValue] = useState<string | undefined>('');
  const imageName = useDebounce(searchValue, 300);
  const { data, error, isLoading, isSuccess } = useRead('getDockerHubPublicImages', {
    query: { imageName },
  });
  const { open } = useTaskSheet('Image');
  const [images, setImages] = useState<DockerHubImageResult[]>();
  const [registryFilter] = useResourceFilter<{ item: any }>('Registry');

  useEffect(() => {
    if (isSuccess && data?.data) {
      setImages(data.data);
    }
  }, [isSuccess, data]);

  const renderImageCard = (image: DockerHubImageResult) => (
    <div
      key={image.name}
      className="p-4 group/versionrow border rounded-lg shadow-sm hover:shadow-md transition-shadow">
      <div className="flex items-center justify-between">
        <a className="hover:underline flex items-center space-x-3" target="_blank" rel="noreferrer" href={image.url!}>
          {image.icon ? (
            <img
              src={image.icon || '/default-icon.png'}
              alt={image.name ?? ''}
              className="h-10 w-10 rounded-full object-cover"
            />
          ) : (
            <Package className="h-8 text-foreground/70" />
          )}

          <div className="flex items-center space-x-2">
            <h5 className="text-lg font-semibold">{image.name}</h5>
            {image.isOfficial && (
              <Tooltip>
                <TooltipTrigger>
                  <Award className="text-green-700 h-5 w-4" />
                </TooltipTrigger>
                <TooltipContent>
                  <p>Official Image</p>
                </TooltipContent>
              </Tooltip>
            )}
          </div>
        </a>
        <div>
          <PullImageBadge
            onClick={() =>
              open({
                kind: 'pull',
                payload: { repository: '', imageTag: image.name ?? '', registryName: registryFilter?.item?.name },
              })
            }
            className="group-hover/versionrow:visible"
          />
        </div>
      </div>
      <p className="text-sm text-foreground/70 mt-2">{image.description || 'No description available.'}</p>
      {!(image.isOfficial && image.pullCount === 0) && (
        <div className="flex justify-between items-center mt-4 text-sm text-foreground/70">
          <div className="flex items-center space-x-1">
            <Star className="h-4 w-4 text-yellow-500" />
            <span>{formatNumber((image.starCount as number) ?? 0)} Stars</span>
          </div>
          <div className="flex items-center space-x-1">
            <Package className="h-4 w-4 text-blue-500" />
            <span>{formatNumber((image.pullCount as number) ?? 0)} Downloads</span>
          </div>
        </div>
      )}
    </div>
  );

  const onSearch = useCallback((searchTerm: string) => {
    setSearchValue(searchTerm);
  }, []);

  return (
    <TooltipProvider>
      <div className="space-y-8">
        <div className="text-center space-y-2">
          <h4 className="text-3xl md:text-4xl font-medium text-slate-900 dark:text-slate-50">Docker Image Registry</h4>
          <p className="text-slate-600 dark:text-slate-400 max-w-2xl mx-auto">
            Search for official Docker images and community contributions.
          </p>
        </div>
        <div className="relative mb-2 sm:mb-0">
          <SearchField onSearch={onSearch} placeholder="Search for Docker images, (e.g., nginx)" />

          {isLoading && <Loader />}
        </div>
        {error && (
          <div className="text-red-500 text-center">
            <p>Failed to fetch images. Please try again later.</p>
          </div>
        )}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6 mt-6">{images?.map(renderImageCard)}</div>
        {!isLoading && images?.length === 0 && (
          <div className="text-center text-foreground/70">
            <p>
              No images found for <b>{searchValue}</b>.
            </p>
          </div>
        )}
      </div>
    </TooltipProvider>
  );
}
