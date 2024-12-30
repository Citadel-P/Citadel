using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using Infrastructure.Services;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Platforms.Commands;

public sealed record StreamContainerLogs(string ContainerId, Guid RequestId, RequestedLogAction Action = RequestedLogAction.START) : ICommand<Result>
{
    internal class Validator : AbstractValidator<StreamContainerLogs>
    {
        public Validator()
        {
            RuleFor(s => s.RequestId).NotNull().NotEmpty();
            RuleFor(s => s.ContainerId).ValidContainerId();
        }
    }
}

internal class StreamContainerLogsHandler(
    IAgentService agentService,
    ApplicationDbContext dbContext)
    : ICommandHandler<StreamContainerLogs, Result>
{
    public async ValueTask<Result> Handle(StreamContainerLogs command, CancellationToken cancellationToken)
    {
        var platformAddress = await dbContext.ContainersInfo
            .AsNoTracking()
            .Include(s => s.Platform)
            .Where(s => s.ContainerId.StartsWith(command.ContainerId))
            .Select(s => s.Platform.Address)
            .FirstOrDefaultAsync(cancellationToken: cancellationToken);

        if (platformAddress == null)
        {
            return Result.Failure(new NotFoundError($"Platform doesn't exist for container {command.ContainerId}"));
        }

        return await agentService.StreamContainerLogs(platformAddress, command.ContainerId, command.RequestId, Convert(command.Action), cancellationToken);
    }

    private static Infrastructure.RequestedLogAction Convert(RequestedLogAction requestedLogAction) 
        => requestedLogAction switch
        {
            RequestedLogAction.START => Infrastructure.RequestedLogAction.START,
            RequestedLogAction.STOP => Infrastructure.RequestedLogAction.STOP,
            _ => throw new NotImplementedException(),
        };
}

public enum RequestedLogAction
{
    START,
    STOP
}