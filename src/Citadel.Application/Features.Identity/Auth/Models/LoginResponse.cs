namespace Application.Features.Identity.Auth.Models;

public enum LoginNextStep
{
    Completed,
    VerifyMfa,
    EnrollMfa
}

public sealed record LoginResponse(string? AccessToken, LoginNextStep NextStep);
