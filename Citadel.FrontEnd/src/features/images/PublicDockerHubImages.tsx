import { useEffect, useState } from 'react';
import { Input } from '@/components/ui/input';
import { Search } from 'lucide-react';
import { useGETPublicDockerImages } from './hooks/useGETPublicDockerImages';
import { DockerHubPublicImage } from '@/api/_generated';
import Loader from '@/components/ui/loader';
import { PullImageBadge } from '@/components/ui/PullImageBadge';
import { useSheetState } from './hooks/useSheetState';
import { Sheet } from '@/components/ui/sheet';
import PullProgressSheetContent from './PullProgressSheetContent';

export function PublicDockerHubImages() {
  const [searchValue, setSearchValue] = useState<string | undefined>(undefined);
  const { data, error, isLoading, isSuccess, refetch } = useGETPublicDockerImages(searchValue);
  const [images, setImages] = useState<DockerHubPublicImage[]>();
  const { sheetState, openSheet, closeSheet } = useSheetState<DockerHubPublicImage>();

  useEffect(() => {
    if (isSuccess && data?.data) {
      setImages(data.data);
    }
  }, [isSuccess, data]);

  function handleSearch() {
    refetch(); // Trigger the query with the current search value
  }

  function handleInputChange(event: React.ChangeEvent<HTMLInputElement>) {
    setSearchValue(event.target.value);
  }

  function handleKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'Enter') {
      handleSearch();
    }
  }

  return (
    <>
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
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6 mt-6">
          {images?.map((image) => (
            <div
              key={image.name}
              className="p-4 group/versionrow border rounded-lg shadow-sm bg-white dark:bg-slate-800 hover:shadow-md transition-shadow">
              <div className="flex items-center justify-between">
                <div className="flex items-center space-x-4">
                  <img
                    src={image.icon || '/default-icon.png'}
                    alt={image.name ?? ''}
                    className="h-10 w-10 rounded-full object-cover"
                  />
                  <h5 className="text-lg font-semibold text-slate-900 dark:text-slate-50">{image.name}</h5>
                </div>
                <div>
                  <PullImageBadge onClick={() => openSheet(image)} className="group-hover/versionrow:visible" />
                </div>
              </div>
              <p className="text-sm text-slate-600 dark:text-slate-400 mt-2">
                {image.description || 'No description available.'}
              </p>
            </div>
          ))}
        </div>
        {!isLoading && images?.length === 0 && (
          <div className="text-center text-slate-600 dark:text-slate-400">
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
              imageTag: sheetState.image.name ?? '',
            }}
          />
        </Sheet>
      )}
    </>
  );
}
