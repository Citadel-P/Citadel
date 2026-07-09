using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Platforms.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Write)]
public sealed record CreateEdgeAgentEnrollment(Guid PlatformId, string CoreUrl) : ICommand<Result<EdgeAgentEnrollmentResult>>
{
    internal sealed class Validator : AbstractValidator<CreateEdgeAgentEnrollment>
    {
        public Validator()
        {
            RuleFor(x => x.PlatformId).NotEmpty();
            RuleFor(x => x.CoreUrl)
                .NotEmpty()
                .Must(x => Uri.TryCreate(x, UriKind.Absolute, out var uri)
                           && (uri.Scheme == Uri.UriSchemeHttp || uri.Scheme == Uri.UriSchemeHttps))
                .WithMessage("CoreUrl must be an absolute HTTP or HTTPS URL.");
        }
    }
}

internal sealed class CreateEdgeAgentEnrollmentHandler(
    IEdgeAgentManagementService edgeAgentManagementService,
    IUserContextAccessor userContextAccessor)
    : ICommandHandler<CreateEdgeAgentEnrollment, Result<EdgeAgentEnrollmentResult>>
{
    private static readonly TimeSpan EnrollmentTtl = TimeSpan.FromHours(24);

    public ValueTask<Result<EdgeAgentEnrollmentResult>> Handle(CreateEdgeAgentEnrollment command, CancellationToken cancellationToken)
        => new(edgeAgentManagementService.CreateEnrollmentAsync(
            command.PlatformId,
            command.CoreUrl.TrimEnd('/'),
            userContextAccessor.Current.ActorId,
            EnrollmentTtl,
            cancellationToken));
}
