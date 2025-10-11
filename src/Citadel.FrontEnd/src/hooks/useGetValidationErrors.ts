import { ProblemDetails, HttpValidationProblemDetails } from '@/api/generated/api.types';

export const useGetValidationErrors = (error: Error | null): string | undefined => {
  if (!error) return undefined;

  const validationProblem = (error as { error?: HttpValidationProblemDetails }).error;

  if (validationProblem?.status === 400) {
    if (validationProblem.errors) {
      // Join all validation error messages into a single string
      return Object.values(validationProblem.errors).flat().join(', ');
    }

    if ((validationProblem as ProblemDetails)?.detail) {
      return validationProblem.detail!;
    }
  }

  const problem = (error as { error?: ProblemDetails }).error;

  if (problem?.status && problem.status > 400 && problem.status <= 499) {
    return problem.detail!;
  }

  return undefined;
};
