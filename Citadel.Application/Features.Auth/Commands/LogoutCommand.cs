using Application.Services;
using Application.Utils;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;

namespace Application.Features.Auth.Commands;

public sealed record LogoutCommand(): ICommand<Result>;

internal sealed class LogoutCommandHandler(
    IJwtService jwtService,
    IHttpContextAccessor context,
    ApplicationDbContext dbContext) : ICommandHandler<LogoutCommand, Result>
{
    public async ValueTask<Result> Handle(LogoutCommand query, CancellationToken cancellationToken)
    {
        var refreshToken = context.HttpContext.Request.Cookies[Constants.RefreshToken];

        if (string.IsNullOrWhiteSpace(refreshToken))
            return Result.Success();

        context.HttpContext.Response.Cookies.Delete(Constants.RefreshToken);

        if (!jwtService.TryValidate(refreshToken, out var tokenId))
            return Result.Success();

        var existing = await dbContext.RefreshTokens.FindAsync(tokenId, cancellationToken);
        if (existing == null)
            return Result.Success();

        dbContext.RefreshTokens.Remove(existing);
        await dbContext.SaveChangesAsync(cancellationToken);

        return Result.Success();
    }
}


