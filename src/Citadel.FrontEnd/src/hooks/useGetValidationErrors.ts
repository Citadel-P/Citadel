import { ProblemDetails } from '@/api/generated/api.types';

export const useGetValidationErrors = (error: Error | null): string | undefined => {
  if (!error) return undefined;

  const problem = (error as { error?: ProblemDetails }).error;
  if (Number(problem?.status) === 400) {
    return getValidationErrors(problem as ProblemDetails);
  }

  if (problem?.status && Number(problem.status) > 400 && Number(problem.status) <= 499) {
    return problem.detail!;
  }

  return undefined;
};

export function getValidationErrors(validationProblem: Pick<ProblemDetails, 'errors'>): string | undefined {
  if (validationProblem.errors) {
    // Join all validation error messages into a single string
    return Object.values(validationProblem.errors).flat().join(', ');
  }
}
