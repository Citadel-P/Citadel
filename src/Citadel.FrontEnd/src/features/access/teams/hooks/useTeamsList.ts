import { useRead } from '@/lib/hooks';
import { useTeamQuery } from '@/lib/atoms';

export const useTeamsList = (teamName?: string | undefined, pageSize?: number | undefined) => {
  const [query] = useTeamQuery();

  const { data, isLoading } = useRead('listTeams', {
    query: {
      Page: query.page,
      PageSize: pageSize ?? query.pageSize,
      Name: teamName ?? query.teamName,
    },
  });

  return {
    pagedUsers: data?.data.pagedResult,
    isLoading,
  };
};
