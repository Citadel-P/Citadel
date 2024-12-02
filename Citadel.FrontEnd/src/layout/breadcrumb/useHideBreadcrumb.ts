import { useAppContext } from '@/AppProvider';
import { useEffect } from 'react';

export const useHideBreadcrumb = () => {
  const { setIsBreadcrumbHidden } = useAppContext();
  useEffect(() => {
    setIsBreadcrumbHidden(true);
    return () => {
      setIsBreadcrumbHidden(false);
    };
  }, []);
};
