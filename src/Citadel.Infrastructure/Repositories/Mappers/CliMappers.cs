using Hosting.Common.ErrorTypes;
using Hosting.DockerClient.Services;
using LightResults;

namespace Infrastructure.Repositories.Mappers;

internal static class CliMappers
{
    internal static Result Map(this ProcessExecutionResult result)
    {
        if (!result.IsSuccess)
        {
            var error = string.IsNullOrEmpty(result.StandardError) ? result.StandardOutput : result.StandardError;
            return Result.Failure(new BadRequestError($"ExitCode={result.ExitCode}. Error={error}"));
        }

        return Result.Success();
    }
}
