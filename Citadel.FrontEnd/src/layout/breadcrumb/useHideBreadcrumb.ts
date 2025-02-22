import { AppContext } from '@/AppProvider';
import { useContextSelector } from 'use-context-selector';
import { useEffect } from 'react';

export const useHideBreadcrumb = () => {
  const setIsBreadcrumbHidden = useContextSelector(AppContext, (v) => v?.setIsBreadcrumbHidden!);
  useEffect(() => {
    setIsBreadcrumbHidden(true);
    return () => {
      setIsBreadcrumbHidden(false);
    };
  }, []);
};
