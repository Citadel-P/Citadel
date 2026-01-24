using Application.Services;
using Domain.Contracts.Interfaces;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Features.Deployments.Commands;

public sealed record DeleteDeployments(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteDeploymentsHandler(
    IServiceScopeFactory scopeFactory,
    IDeploymentProcessingService deploymentProcessingService)
    : ICommandHandler<DeleteDeployments, Result>
{
    public async ValueTask<Result> Handle(DeleteDeployments command, CancellationToken cancellationToken)
    {
        var deployments = await deploymentProcessingService.MarkProcessingAsync(command.Ids, cancellationToken);

        if (deployments.Count == 0)
        {
            return Result.Failure(new NotFoundError("No deployments found matching the provided IDs for deletion."));
        }

        await deploymentProcessingService.NotifyProcessingAsync(deployments, cancellationToken);

        var deleted = await DeleteAsync(command.Ids, cancellationToken);
        if (deleted <= 0)
        {
            await deploymentProcessingService.RollbackProcessingAsync(deployments, cancellationToken);
            return Result.Failure(new NotFoundError("No deployments found matching the provided IDs for deletion."));
        }

        return Result.Success();
    }

    private async Task<int> DeleteAsync(IEnumerable<Guid> ids, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var deleted = await uow.Deployments.RemoveRangeAsync(ids, ct);
        await uow.CommitAsync(ct);

        return deleted;
    }
}

