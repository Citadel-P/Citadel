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
public sealed record PatchTag(Guid Id, string? Name, string? Color) : ICommand<Result<Tag>>
{
    internal sealed class Validator : AbstractValidator<PatchTag>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();

            When(x => x.Name is not null, () =>
            {
                RuleFor(x => x.Name!)
                    .NotEmpty()
                    .Must(name => name.Trim().Length <= TagValidation.MaxNameLength)
                    .WithMessage($"Tag name cannot exceed {TagValidation.MaxNameLength} characters.");
            });

            When(x => x.Color is not null, () =>
            {
                RuleFor(x => x.Color!)
                    .NotEmpty()
                    .Must(TagValidation.IsValidColor)
                    .WithMessage("Tag color must be a valid hex color in #RRGGBB format.");
            });
        }
    }
}

internal sealed class PatchTagHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<PatchTag, Result<Tag>>
{
    public async ValueTask<Result<Tag>> Handle(PatchTag command, CancellationToken cancellationToken)
    {
        var existing = await unitOfWork.Tags.GetAsync(command.Id, cancellationToken);
        if (existing is null)
            return Result.Failure<Tag>(new NotFoundError("Tag not found."));

        var name = command.Name ?? existing.Name;
        var color = command.Color ?? existing.Color;
        var normalizedName = TagValidation.NormalizeName(name);
        if (await unitOfWork.Tags.ExistsByNormalizedNameExceptAsync(normalizedName, command.Id, cancellationToken))
            return Result.Failure<Tag>(new ConflictError("A tag with this name already exists."));

        var updated = existing.RenameAndRecolor(name, color);
        await unitOfWork.Tags.UpdateAsync(updated, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return updated;
    }
}
