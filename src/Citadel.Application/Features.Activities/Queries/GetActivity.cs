using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Activities.Queries;

public record GetActivity(Guid Id) : IQuery<Result<ActivityEvent>>;

internal sealed class GetActivityHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor) : IQueryHandler<GetActivity, Result<ActivityEvent>>
{
    public async ValueTask<Result<ActivityEvent>> Handle(GetActivity query, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        var activity = user.IsAdmin
            ? await unitOfWork.ActivityEventRepository.GetByIdAsync(query.Id, cancellationToken)
            : await unitOfWork.ActivityEventRepository.GetAuthorizedByIdAsync(query.Id, user.ActorId, cancellationToken);

        return activity ?? Result.Failure<ActivityEvent>(new NotFoundError("Activity does not exist"));
    }
}
