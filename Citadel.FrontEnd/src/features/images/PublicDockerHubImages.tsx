import { useEffect, useState } from 'react';
import { Input } from '@/components/ui/input';
import { Search, Package, Award, Star } from 'lucide-react';
import { useGETPublicDockerImages } from './hooks/useGETPublicDockerImages';
import { DockerHubImageModel } from '@/api/_generated';
import { PullImageBadge } from '@/components/ui/PullImageBadge';
import { useSheetState } from './hooks/useSheetState';
import { Sheet } from '@/components/ui/sheet';
import PullProgressSheetContent from './PullProgressSheetContent';
import Loader from '@/components/ui/loader';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { formatNumber } from '@/lib/utils';

export function PublicDockerHubImages() {
  const [searchValue, setSearchValue] = useState<string | undefined>(undefined);
  const { data, error, isLoading, isSuccess, refetch } = useGETPublicDockerImages(searchValue);
  const [images, setImages] = useState<DockerHubImageModel[]>();
  const { sheetState, openSheet, closeSheet } = useSheetState<DockerHubImageModel>();

  useEffect(() => {
    if (isSuccess && data?.data) {
      setImages(data.data);
    }
  }, [isSuccess, data]);

  const handleSearch = () => refetch();

  const handleInputChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setSearchValue(event.target.value);
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter') {
      handleSearch();
    }
  };

  const renderImageCard = (image: DockerHubImageModel) => (
    <div
      key={image.repo_name}
      className="p-4 group/versionrow border rounded-lg shadow-sm hover:shadow-md transition-shadow">
      <div className="flex items-center justify-between">
        <a className="hover:underline flex items-center space-x-3" target="_blank" rel="noreferrer" href={image.url!}>
          {image.icon ? (
            <img
              src={image.icon || '/default-icon.png'}
              alt={image.repo_name ?? ''}
              className="h-10 w-10 rounded-full object-cover"
            />
          ) : (
            <Package className="h-8 text-foreground/70" />
          )}

          <div className="flex items-center space-x-2">
            <h5 className="text-lg font-semibold">{image.repo_name}</h5>
            {image.is_official && (
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
          <PullImageBadge onClick={() => openSheet(image)} className="group-hover/versionrow:visible" />
        </div>
      </div>
      <p className="text-sm text-foreground/70 mt-2">{image.short_description || 'No description available.'}</p>
      {!(image.is_official && image.pull_count === 0) && (
        <div className="flex justify-between items-center mt-4 text-sm text-foreground/70">
          <div className="flex items-center space-x-1">
            <Star className="h-4 w-4 text-yellow-500" />
            <span>{formatNumber(image.star_count ?? 0)} Stars</span>
          </div>
          <div className="flex items-center space-x-1">
            <Package className="h-4 w-4 text-blue-500" />
            <span>{formatNumber(image.pull_count ?? 0)} Downloads</span>
          </div>
        </div>
      )}
    </div>
  );

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
          <Input
            type="search"
            placeholder="Search for Docker images, (e.g., nginx)"
            value={searchValue}
            onChange={handleInputChange}
            onKeyDown={handleKeyDown}
            className="bg-background placeholder:text-foreground/50 h-9 px-5 pr-10 shadow-xs rounded-full text-xs focus:outline-hidden focus-visible:ring-offset-0"
          />
          <Search
            className="absolute text-slate-300 right-0 top-0 mt-1.5 mr-4 h4 w-4 hover:cursor-pointer hover:text-slate-500"
            onClick={handleSearch}
          />
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
      {sheetState.image && (
        <Sheet open={sheetState.isOpen} onOpenChange={(open) => (open ? openSheet(sheetState.image!) : closeSheet())}>
          <PullProgressSheetContent
            sheetProps={{
              repository: '',
              imageTag: sheetState.image.repo_name ?? '',
            }}
          />
        </Sheet>
      )}
    </TooltipProvider>
  );
}
