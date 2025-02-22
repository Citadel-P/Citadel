import { Input } from '@/components/ui/input';
import { Search } from 'lucide-react';

export const SearchField = () => {
  return (
    <div className="relative font-semibold mb-2 sm:mb-0">
      <Input
        type="search"
        placeholder="Search"
        className="bg-background placeholder:text-foreground/50 h-9 px-5 pr-10 rounded-full text-xs focus:outline-hidden focus-visible:ring-transparent"
      />
      <Search className="absolute text-slate-300 right-0 top-0 mt-1.5 mr-4 h4 w-4" />
    </div>
  );
};
