using Domain.Contracts.Interfaces;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

public sealed record DeleteDeployments(IEnumerable<Guid> Ids) : ICommand<Result>;

internal class DeleteDeploymentsHandler(IUnitOfWork unitOfWork) : ICommandHandler<DeleteDeployments, Result>
{
    public async ValueTask<Result> Handle(DeleteDeployments command, CancellationToken cancellationToken)
    {
        var result = await unitOfWork.Deployments.RemoveRangeAsync(command.Ids, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return result > 0
            ? Result.Success()
            : Result.Failure(new NotFoundError("No deployments found matching the provided IDs for deletion."));
    }
}

