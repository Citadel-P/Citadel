import { useState } from 'react';
import { Input } from '@/components/ui/input';
import { Search } from 'lucide-react';

export function PublicDockerHubImages() {
  const [searchValue, setSearchValue] = useState('');

  function handleClick() {
    console.log('Current Input Value:', searchValue);
  }

  function handleInputChange(event: React.ChangeEvent<HTMLInputElement>) {
    setSearchValue(event.target.value);
  }
  function handleKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'Enter') {
      handleClick();
    }
  }

  return (
    <>
      <div className="max-w-6xl mx-auto space-y-8">
        <div className="text-center space-y-2">
          <h4 className="text-3xl md:text-4xl font-medium text-slate-900 dark:text-slate-50">Docker Image Registry</h4>
          <p className="text-slate-600 dark:text-slate-400 max-w-2xl mx-auto">
            Search for official Docker images, community contributions, and verified publishers
          </p>
        </div>
      </div>
      <div className="relative mb-2 sm:mb-0">
        <Input
          type="search"
          placeholder="Search for Docker images, (eg nginx)"
          value={searchValue}
          onChange={handleInputChange}
          onKeyDown={handleKeyDown}
          className="bg-background placeholder:text-foreground/50 h-9 px-5 pr-10 shadow-xs rounded-full text-xs focus:outline-hidden focus-visible:ring-offset-0"
        />
        <Search
          className="absolute text-slate-300 right-0 top-0 mt-1.5 mr-4 h4 w-4 hover:cursor-pointer hover:text-slate-500"
          onClick={handleClick}
        />
      </div>
    </>
  );
}
