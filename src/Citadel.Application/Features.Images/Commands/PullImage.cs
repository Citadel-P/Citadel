using Application.Services;
using Hosting.Common;
using Domain.Contracts.Resources.Images;
using FluentValidation;
using Hosting.Common.Attributes;
using Mediator;
using System.Runtime.CompilerServices;

namespace Application.Features.Images.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Pull)]
public sealed record PullImage(Guid PlatformId, Guid RegistryId, string ImageTag) : IStreamCommand<PullImageStreamItem>
{
    internal class Validator : AbstractValidator<PullImage>
    {
        public Validator()
        {
            RuleFor(s => s.ImageTag).NotEmpty().NotNull();
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.RegistryId).NotEmpty().NotNull();
        }
    }
}

internal sealed class PullImageHandler(IPullImageService pullImageService) : IStreamCommandHandler<PullImage, PullImageStreamItem>
{
    public async IAsyncEnumerable<PullImageStreamItem> Handle(PullImage command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var item in pullImageService.PullAsync(new PullImageService.PullImageInput(
            ImageTag: command.ImageTag,
            PlatformId: command.PlatformId,
            RegistryId: command.RegistryId
            ), cancellationToken))
        {
            yield return item;
        }
    }
}