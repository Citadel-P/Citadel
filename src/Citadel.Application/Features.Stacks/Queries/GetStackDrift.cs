using Application.Services;
using Domain;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Queries;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read)]
public sealed record GetStackDrift(Guid Id) : IQuery<Result<StackDriftReport>>;

internal sealed class GetStackDriftHandler(
    IStackDriftChecker driftChecker) : IQueryHandler<GetStackDrift, Result<StackDriftReport>>
{
    public async ValueTask<Result<StackDriftReport>> Handle(GetStackDrift query, CancellationToken cancellationToken)
    {
        try
        {
            return await driftChecker.CheckAsync(query.Id, cancellationToken);
        }
        catch (InvalidOperationException ex)
        {
            return Result.Failure<StackDriftReport>(new BadRequestError(ex.Message));
        }
    }
}
