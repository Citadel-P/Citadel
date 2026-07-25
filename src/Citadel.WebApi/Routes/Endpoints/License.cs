using Application.Features.Licensing;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Licensing;

namespace WebApi.Routes.Endpoints;

public static class License
{
    public static async Task<Results<Ok<LicenseView>, ProblemHttpResult>> Get(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetLicense(), cancellationToken);
        return EndpointHandlers.HandleResult(result, LicenseView.Map);
    }

    public static async Task<Results<Ok<LicenseEntitlementsView>, ProblemHttpResult>> GetEntitlements(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetLicenseEntitlements(), cancellationToken);
        return EndpointHandlers.HandleResult(result, LicenseEntitlementsView.Map);
    }

    public static async Task<Results<Ok<LicenseView>, ProblemHttpResult>> Install(
        IMediator mediator,
        [FromBody] InstallLicenseInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new InstallLicense(input.License), cancellationToken);
        return EndpointHandlers.HandleResult(result, LicenseView.Map);
    }

    public static async Task<Results<Ok<LicenseView>, ProblemHttpResult>> Remove(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RemoveLicense(), cancellationToken);
        return EndpointHandlers.HandleResult(result, LicenseView.Map);
    }

    public static async Task<Results<Ok<LicenseRequestView>, ProblemHttpResult>> GetRequest(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetLicenseRequest(), cancellationToken);
        return EndpointHandlers.HandleResult(result, LicenseRequestView.Map);
    }
}
