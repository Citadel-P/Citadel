import { useRead } from '@/lib/hooks';
import { useUserQuery } from '@/lib/atoms';

export const useUsersList = (userName?: string, pageSize?: number) => {
  const [query] = useUserQuery();

  const { data, isLoading } = useRead('listUsers', {
    query: {
      Page: query.page,
      PageSize: pageSize ?? query.pageSize,
      Name: userName ?? query.userName,
    },
  });

  return {
    pagedUsers: data?.data.pagedResult,
    isLoading,
  };
};
