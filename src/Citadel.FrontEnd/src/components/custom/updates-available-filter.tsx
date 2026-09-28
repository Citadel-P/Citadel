import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { ListFilter } from 'lucide-react';
import { useSearchParams } from 'react-router';

const UPDATE_FILTER_KEY = 'updates';
const UPDATE_FILTER_VALUE = 'available';

export const useUpdatesAvailableFilter = () => {
  const [searchParams, setSearchParams] = useSearchParams();
  const updatesAvailableOnly = searchParams.get(UPDATE_FILTER_KEY) === UPDATE_FILTER_VALUE;

  const setUpdatesAvailableOnly = (enabled: boolean) => {
    setSearchParams((current) => {
      const next = new URLSearchParams(current);
      if (enabled) {
        next.set(UPDATE_FILTER_KEY, UPDATE_FILTER_VALUE);
        next.delete('overview');
      } else {
        next.delete(UPDATE_FILTER_KEY);
      }
      return next;
    });
  };

  return { updatesAvailableOnly, setUpdatesAvailableOnly };
};

export const UpdatesAvailableFilter = () => {
  const { updatesAvailableOnly, setUpdatesAvailableOnly } = useUpdatesAvailableFilter();

  return (
    <Button
      type="button"
      variant="outline"
      aria-pressed={updatesAvailableOnly}
      className={cn(
        'h-(--control-height) shrink-0 rounded-sm px-2.5',
        updatesAvailableOnly && 'border-primary/40 bg-primary/10 text-primary hover:bg-primary/15',
      )}
      onClick={() => setUpdatesAvailableOnly(!updatesAvailableOnly)}>
      <ListFilter className="size-3.5" />
      Updates available
    </Button>
  );
};
