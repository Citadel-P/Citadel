import { Input } from '@/components/ui/input';
import { useDebounce } from '@/hooks/useDebounce';
import { cn } from '@/lib/utils';
import { Search } from 'lucide-react';
import { useState, useEffect, useCallback } from 'react';

interface SearchFieldProps {
  className?: string;
  placeholder?: string;
  onSearch: (searchTerm: string) => void;
}

export const SearchField = ({ onSearch, placeholder = 'Search', className = '' }: SearchFieldProps) => {
  const [searchTerm, setSearchTerm] = useState('');
  const debouncedSearchTerm = useDebounce(searchTerm, 300);

  const handleSearchChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setSearchTerm(e.target.value);
  }, []);

  useEffect(() => {
    onSearch(debouncedSearchTerm);
  }, [debouncedSearchTerm, onSearch]);

  return (
    <div className={cn(`relative mb-2 sm:mb-0`, className)}>
      <Input
        type="search"
        placeholder={placeholder}
        value={searchTerm}
        onChange={handleSearchChange}
        className="bg-background placeholder:text-foreground/50 h-9 px-5 pr-10 rounded-full text-sm focus:outline-hidden focus-visible:ring-transparent"
        aria-label="Search resources by name"
      />
      <Search className="absolute text-slate-300 right-0 top-0 mt-1.5 mr-4 h4 w-4 pointer-events-none" />
    </div>
  );
};
