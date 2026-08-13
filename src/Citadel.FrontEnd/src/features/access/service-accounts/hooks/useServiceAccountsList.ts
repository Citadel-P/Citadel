import { useRead } from '@/lib/hooks';
import { useUserQuery } from '@/lib/atoms';

export const useServiceAccountsList = (name?: string, pageSize?: number) => {
  const [query] = useUserQuery();
  const { data, isLoading } = useRead('listServiceAccounts', {
    query: {
      Page: query.page,
      PageSize: pageSize ?? query.pageSize,
      Name: name ?? query.userName,
      IncludeArchived: false,
    },
  });
  return { pagedServiceAccounts: data?.data.pagedResult, isLoading };
};
