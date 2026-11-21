import { useQueryReconciliation, useResourceReconciliation } from '@/hooks/useQueryReconciliation';

export function useReconcileBuildProject() {
  return useResourceReconciliation('build-projects', 'listBuildProjects', 'build-project', 'getBuildProject');
}

export function useReconcileBuildRun() {
  const reconcile = useQueryReconciliation();
  const project = useReconcileBuildProject();
  return async (projectId: string, runId: string) => {
    await Promise.all([
      project(projectId),
      reconcile(`build-runs:${projectId}`, 'listBuildRuns', { query: { projectId } }),
      reconcile(`build-run:${runId}`, 'getBuildRun', { id: runId }),
    ]);
  };
}
