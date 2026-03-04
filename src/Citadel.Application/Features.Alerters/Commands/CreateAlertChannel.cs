using Application.Services.Alerts;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using FluentValidation;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.Alerters.Commands;

public sealed record CreateAlertChannel(
    string Name,
    AlertDestination AlertDestination,
    string Url,
    bool IsActive) : ICommand<Result<AlertChannel>>
{
    internal sealed class Validator : AbstractValidator<CreateAlertChannel>
    {
        public Validator()
        {
            RuleFor(x => x.Url).NotEmpty();
        }
    }
}

internal sealed class CreateAlertChannelHandler(
    IUnitOfWork unitOfWork,
    AlertRuleCache alertRuleCache,
    IHttpContextAccessor httpContextAccessor)
    : ICommandHandler<CreateAlertChannel, Result<AlertChannel>>
{
    public async ValueTask<Result<AlertChannel>> Handle(CreateAlertChannel command, CancellationToken cancellationToken)
    {
        var user = httpContextAccessor.HttpContext?.User
            ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var channel = new AlertChannel(command.Name, command.AlertDestination, command.Url, command.IsActive, user.GetActorId());
        await unitOfWork.AlertRules.AddChannelAsync(channel, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        alertRuleCache.UpsertChannel(channel);

        return channel;
    }
}
