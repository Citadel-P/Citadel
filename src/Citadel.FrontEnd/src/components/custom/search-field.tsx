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
        className="bg-background placeholder:text-muted-foreground h-(--control-height) pl-3 pr-9 rounded-md text-sm"
        aria-label="Search resources by name"
      />
      <Search className="absolute text-muted-foreground right-3 top-1/2 -translate-y-1/2 size-4 pointer-events-none" />
    </div>
  );
};
