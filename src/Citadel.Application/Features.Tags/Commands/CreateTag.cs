using Domain.Contracts.Interfaces;
using Domain.Entities.Tags;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Tags.Commands;

[RequirePermission(ResourceType.Tag, PermissionLevel.Write)]
public sealed record CreateTag(string Name, string Color) : ICommand<Result<Tag>>
{
    internal sealed class Validator : AbstractValidator<CreateTag>
    {
        public Validator()
        {
            RuleFor(x => x.Name)
                .NotEmpty()
                .Must(name => name.Trim().Length <= TagValidation.MaxNameLength)
                .WithMessage($"Tag name cannot exceed {TagValidation.MaxNameLength} characters.");

            RuleFor(x => x.Color)
                .NotEmpty()
                .Must(TagValidation.IsValidColor)
                .WithMessage("Tag color must be a valid hex color in #RRGGBB format.");
        }
    }
}

internal sealed class CreateTagHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext)
    : ICommandHandler<CreateTag, Result<Tag>>
{
    public async ValueTask<Result<Tag>> Handle(CreateTag command, CancellationToken cancellationToken)
    {
        var user = userContext.Current;
        if (user is null || user.ActorId == Guid.Empty)
            return Result.Failure<Tag>(new UnauthorizedError("Missing user context."));

        var normalizedName = TagValidation.NormalizeName(command.Name);
        if (await unitOfWork.Tags.ExistsByNormalizedNameAsync(normalizedName, cancellationToken))
            return Result.Failure<Tag>(new ConflictError("A tag with this name already exists."));

        var tag = Tag.Create(command.Name, command.Color, user.ActorId);
        await unitOfWork.Tags.AddAsync(tag, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return tag;
    }
}
