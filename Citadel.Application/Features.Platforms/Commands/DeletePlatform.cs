using Hosting.Common.ErrorTypes;
using Infrastructure.Entities;
using FluentValidation;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Infrastructure.EntityFramework;
using Infrastructure.Services;

namespace Application.Features.Platforms.Commands;

public sealed record DeletePlatform(Guid Id) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<DeletePlatform>
    {
        public Validator()
            => RuleFor(s => s.Id).NotNull();
    }
}

internal class DeletePlatformHandler(ApplicationDbContext dbContext, ICacheService cacheService) : ICommandHandler<DeletePlatform, Result>
{
    public async ValueTask<Result> Handle(DeletePlatform command, CancellationToken cancellationToken)
    {
        Platform platform = await dbContext.Platforms
            .AsNoTracking()
            .Include(s => s.SystemInfo)
            .SingleOrDefaultAsync(s => s.Id == command.Id, cancellationToken);

        if (platform is null)
        {
            return Result.Fail(new NotFoundError("Platform does not exists"));
        }

        dbContext.Platforms.Remove(platform);
        await dbContext.SaveChangesAsync(cancellationToken);

        cacheService.DeletePlatformId(platform.SystemInfo.DaemonId);

        return Result.Ok();
    }
}