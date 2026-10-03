import type { AdoptionIssue } from '@/api/generated/api.types';

const sensitiveEnvironmentIssueCode = 'SENSITIVE_ENVIRONMENT_VALUE_REQUIRED';
const environmentFieldPrefix = 'spec.environmentVariables.';
const environmentNamePattern = /^[A-Za-z_][A-Za-z0-9_]*$/;

export const getDeploymentConfigurationNames = (
  persistedNames: Iterable<string>,
  adoptionIssues: readonly AdoptionIssue[],
  includePendingAdoptionSecrets: boolean,
): string[] => {
  const names = new Set(persistedNames);

  if (includePendingAdoptionSecrets) {
    for (const issue of adoptionIssues) {
      if (issue.code !== sensitiveEnvironmentIssueCode || !issue.fieldPath?.startsWith(environmentFieldPrefix)) {
        continue;
      }

      const name = issue.fieldPath.slice(environmentFieldPrefix.length);
      if (environmentNamePattern.test(name)) {
        names.add(name);
      }
    }
  }

  return [...names].sort();
};
