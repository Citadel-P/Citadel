using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Activities.Queries;

public record GetActivity(Guid Id) : IQuery<Result<ActivityEvent>>;

internal sealed class GetActivityHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetActivity, Result<ActivityEvent>>
{
    public async ValueTask<Result<ActivityEvent>> Handle(GetActivity query, CancellationToken cancellationToken)
    {
        var activity = await unitOfWork.ActivityEventRepository.GetByIdAsync(query.Id, cancellationToken);
        return activity ?? Result.Failure<ActivityEvent>(new NotFoundError("Activity does not exist"));

    }
}