using Domain.Contracts.Interfaces;
using Hosting.DockerClient.Services;
using System.Runtime.CompilerServices;

namespace Infrastructure.Repositories;

internal sealed class AutomationProcessRunner(ICommandExecutor commandExecutor) : IAutomationProcessRunner
{
    public async IAsyncEnumerable<AutomationProcessOutput> StreamAsync(
        string fileName,
        IEnumerable<string> arguments,
        IDictionary<string, string>? environmentVariables,
        string workingDirectory,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var output in commandExecutor.StreamAsync(
                           fileName,
                           arguments,
                           environmentVariables,
                           workingDirectory,
                           cancellationToken: cancellationToken))
        {
            yield return new AutomationProcessOutput(output.StdOut, output.StdErr, output.ExitCode);
        }
    }
}
