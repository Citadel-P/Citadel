using Infrastructure.Entities;
using Hosting.Common.ErrorTypes;
using FluentValidation;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using Hosting.Common;
using Infrastructure.EntityFramework;
using Infrastructure.Services;

namespace Application.Features.Platforms.Commands;

/// <summary>
/// Create or update a platform
/// </summary>
public sealed record UpsertPlatform(Guid? Id, string Name, string Address) : ICommand<Result<Platform>>
{
    internal class Validator : AbstractValidator<UpsertPlatform>
    {
        public Validator()
        {
            RuleFor(x => x.Name).NotEmpty().Length(4, 128);
            RuleFor(x => x.Address).ValidHostOrIp();
        }
    }
}

internal class UpsertPlatformHandler(
    IAgentService agentService,
    ApplicationDbContext dbContext,
    ILogger<UpsertPlatformHandler> logger)
    : ICommandHandler<UpsertPlatform, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(UpsertPlatform command, CancellationToken cancellationToken)
    {
        return (command.Id is null)
                ? await CreatePlatform(command, cancellationToken)
                : await UpdatePlatform(command, cancellationToken);
    }

    private async Task<Result<Platform>> CreatePlatform(UpsertPlatform command, CancellationToken cancellationToken)
    {
        // Check if the platform already exists
        if (await dbContext.Platforms.AsNoTracking()
                                    .FirstOrDefaultAsync(s => s.Address == command.Address || s.Name == command.Name, cancellationToken: cancellationToken) != null)
        {
            return Result.Fail<Platform>(new ConflictError("A platform with the same [Name] or [Address] already exists!"));
        }

        // Try to get platform system info
        var result = await agentService.GetSystemInfo(command.Address, cancellationToken);
        if (!result.IsSuccess)
        {
            return Result.Fail<Platform>(result.Error);
        }

        var systemInfo = result.Value.SystemInfo.Map();
        systemInfo.SetDaemonId(result.Value.DaemonId);

        var platform = Platform.Create(command.Name, command.Address, systemInfo);

        await dbContext.Platforms.AddAsync(platform, cancellationToken);

        // Save to db
        await dbContext.SaveChangesAsync(cancellationToken);

        logger.LogInformation("A new platform has been added, id = {PlatformId}", platform.Id);
        return Result.Ok(platform);
    }

    private async Task<Result<Platform>> UpdatePlatform(UpsertPlatform command, CancellationToken cancellationToken)
    {
        var platform = await dbContext.Platforms.FirstOrDefaultAsync(s => s.Id == command.Id, cancellationToken);
        if (platform == null)
        {
            Result.Fail<Platform>(new NotFoundError("The provided platform Id does not exists"));
        }

        platform.Update(command.Name, command.Address);

        // Save to db
        await dbContext.SaveChangesAsync(cancellationToken);

        logger.LogInformation("The platform with id = {PlatformId} has been updated", platform.Id);
        return Result.Ok(platform);
    }
}