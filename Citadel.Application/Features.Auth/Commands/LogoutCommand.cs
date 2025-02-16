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
            return Result.Failure(new BadRequestError("Please include a refresh token in the request."));

        if (!jwtService.TryValidate(refreshToken, out var tokenId))
            return Result.Failure(new BadRequestError("Invalid refresh token."));

        var existing = await dbContext.RefreshTokens.FindAsync(tokenId, cancellationToken);
        if (existing == null)
            return Result.Failure(new BadRequestError("Refresh token does not exist."));

        dbContext.RefreshTokens.Remove(existing);
        await dbContext.SaveChangesAsync(cancellationToken);

        context.HttpContext.Response.Cookies.Delete(Constants.RefreshToken);

        return Result.Success();
    }
}


