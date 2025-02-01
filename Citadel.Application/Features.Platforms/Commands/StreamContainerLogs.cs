using FluentValidation;
using Hosting.Common;
using Infrastructure.TaskJobs;
using LightResults;
using Mediator;
using Microsoft.Extensions.Logging;
using Quartz;

namespace Application.Features.Platforms.Commands;

public sealed record StreamContainerLogs(string ContainerId, Guid RequestId, RequestedLogAction RequestedLogAction) : ICommand<Result>
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

internal class StreamContainerLogsHandler(ISchedulerFactory schedulerFactory, ILogger<StreamContainerLogsHandler> logger)
    : ICommandHandler<StreamContainerLogs, Result>
{
    public async ValueTask<Result> Handle(StreamContainerLogs query, CancellationToken cancellationToken)
    {
        var scheduler = await schedulerFactory.GetScheduler(cancellationToken);
        if (query.RequestedLogAction == RequestedLogAction.START)
        {
            await scheduler.EnqueueContainerLogsJob(query.ContainerId, query.RequestId, logger, cancellationToken);
        }
        else
        {
            await scheduler.AbortContainerLogsJob(query.RequestId, logger, cancellationToken);
        }
        return Result.Success();
    }
}

public enum RequestedLogAction
{
    START,
    STOP
}