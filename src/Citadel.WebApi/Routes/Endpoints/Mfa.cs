using Application.Features.Identity.Mfa.Commands;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources.Identity.Mfa;

namespace WebApi.Routes.Endpoints;

public static class Mfa
{
    public static async Task<Results<Ok<MfaVerificationView>, ProblemHttpResult>> VerifyAuthenticationMfa(
        IMediator mediator,
        [FromBody] MfaVerificationInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToVerifyCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, MfaVerificationView.Map);
    }

    public static async Task<Results<Ok<MandatoryMfaSetupView>, ProblemHttpResult>> GetAuthenticationMfaSetup(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetMandatoryMfaSetup(), cancellationToken);
        return EndpointHandlers.HandleResult(result, MandatoryMfaSetupView.Map);
    }

    public static async Task<Results<Ok<MandatoryMfaSetupCompleteView>, ProblemHttpResult>> ConfirmAuthenticationMfaSetup(
        IMediator mediator,
        [FromBody] ConfirmMandatoryMfaSetupInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, MandatoryMfaSetupCompleteView.Map);
    }

    public static async Task<Results<Ok<ProfileMfaStatusView>, ProblemHttpResult>> GetProfileMfaStatus(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetMfaStatus(), cancellationToken);
        return EndpointHandlers.HandleResult(result, ProfileMfaStatusView.Map);
    }

    public static async Task<Results<Ok<ProfileMfaSetupView>, ProblemHttpResult>> StartProfileMfaSetup(
        IMediator mediator,
        [FromBody] StartProfileMfaSetupInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, ProfileMfaSetupView.Map);
    }

    public static async Task<Results<Ok<ProfileMfaRecoveryCodesView>, ProblemHttpResult>> ConfirmProfileMfaSetup(
        IMediator mediator,
        [FromBody] ConfirmProfileMfaSetupInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, ProfileMfaRecoveryCodesView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> DisableProfileMfa(
        IMediator mediator,
        [FromBody] DisableProfileMfaInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<ProfileMfaRecoveryCodesView>, ProblemHttpResult>> RegenerateProfileMfaRecoveryCodes(
        IMediator mediator,
        [FromBody] RegenerateProfileMfaRecoveryCodesInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, ProfileMfaRecoveryCodesView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> ResetUserMfa(
        IMediator mediator,
        [FromRoute][Description("User ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new Application.Features.Identity.Mfa.Commands.ResetUserMfa(id), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }
}
