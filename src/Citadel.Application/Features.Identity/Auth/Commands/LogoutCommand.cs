using Application.Services;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;

namespace Application.Features.Identity.Auth.Commands;

public sealed record LogoutCommand(): ICommand<Result>;

internal sealed class LogoutCommandHandler(IUnitOfWork unitOfWork, IJwtService jwtService,IHttpContextAccessor context) 
    : ICommandHandler<LogoutCommand, Result>
{
    public async ValueTask<Result> Handle(LogoutCommand query, CancellationToken cancellationToken)
    {
        var refreshToken = context.HttpContext?.Request.Cookies[Constants.RefreshToken];

        if (string.IsNullOrWhiteSpace(refreshToken))
            return Result.Success();

        context.HttpContext?.Response.Cookies.Delete(Constants.RefreshToken);

        if (!jwtService.TryValidate(refreshToken, out var tokenId))
            return Result.Success();

        await unitOfWork.RefreshTokens.DeleteAsync(tokenId, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success();
    }
}


