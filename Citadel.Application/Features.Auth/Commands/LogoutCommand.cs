using Application.Services;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Auth.Commands;

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

        var existing = await unitOfWork.RefreshTokens.Query().AsNoTracking().FirstOrDefaultAsync(s => s.Id == tokenId, cancellationToken);
        if (existing == null)
            return Result.Success();

        unitOfWork.RefreshTokens.Remove(existing);
        await unitOfWork.SaveChangesAsync(cancellationToken);

        return Result.Success();
    }
}


