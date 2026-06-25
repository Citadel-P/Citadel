using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities.Git;

namespace Application.Services.SignalR;

public interface IGitRepositoryStreamManager : IStreamGroupManager
{
    Task SendGitRepoInfo(GitRepository repository, string action = "update");
}

internal sealed class GitRepositoryStreamManager(IApplicationHubDispatcher dispatcher) : BaseStreamManager<StreamContext>, IGitRepositoryStreamManager
{
    public Task SendGitRepoInfo(GitRepository repository, string action = "update")
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendGitRepoInfo(repository, action);
    }
}
