using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services;

internal sealed class StackWebhookDeployQueueService(
    IServiceScopeFactory scopeFactory,
    TimeProvider timeProvider)
{
    internal async Task<IReadOnlyList<Guid>> GetReadyIdsAsync(
        int limit,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var ids = await uow.StackWebhookDeployQueue.GetReadyIdsAsync(
            limit,
            timeProvider.GetUtcNow().UtcDateTime,
            cancellationToken);
        await uow.CommitAsync(cancellationToken);
        return ids;
    }

    internal async Task<StackWebhookDeployQueueItem?> TryClaimAsync(
        Guid id,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var item = await uow.StackWebhookDeployQueue.TryClaimAsync(
            id,
            timeProvider.GetUtcNow().UtcDateTime,
            cancellationToken);
        await uow.CommitAsync(cancellationToken);
        return item;
    }

    internal async Task RetryAsync(
        Guid id,
        string reason,
        DateTime availableAt,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.StackWebhookDeployQueue.RetryAsync(
            id,
            reason,
            availableAt,
            cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }

    internal async Task DeleteAsync(Guid id, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.StackWebhookDeployQueue.DeleteAsync(id, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }

    internal async Task RequeueInterruptedAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.StackWebhookDeployQueue.RequeueInterruptedAsync(
            timeProvider.GetUtcNow().UtcDateTime,
            cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }
}
