using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Tags.Commands;

[RequirePermission(ResourceType.Tag, PermissionLevel.Write)]
public sealed record DeleteTag(Guid Id) : ICommand<Result>;

internal sealed class DeleteTagHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<DeleteTag, Result>
{
    public async ValueTask<Result> Handle(DeleteTag command, CancellationToken cancellationToken)
    {
        var existing = await unitOfWork.Tags.GetAsync(command.Id, cancellationToken);
        if (existing is null)
            return Result.Failure(new NotFoundError("Tag not found."));

        await unitOfWork.Tags.DeleteAsync(command.Id, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success();
    }
}
