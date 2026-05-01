import { useEffect, useState } from 'react';
import { PagedResultViewOfUserView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { useUserQuery } from '@/lib/atoms';

export const useUsersList = (userName?: string | undefined, pageSize?: number | undefined) => {
  const [query] = useUserQuery();

  const { data, isLoading } = useRead('listUsers', {
    query: {
      Page: query.page,
      PageSize: pageSize ?? query.pageSize,
      Name: userName ?? query.userName,
    },
  });

  const [pagedUsers, setPagedUsers] = useState<PagedResultViewOfUserView | undefined>();

  useEffect(() => {
    if (!data) return;
    setPagedUsers(data.data.pagedResult);
  }, [data]);

  return { pagedUsers, isLoading };
};
