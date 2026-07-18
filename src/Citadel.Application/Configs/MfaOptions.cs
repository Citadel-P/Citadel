using Domain;

namespace Application.Configs;

public sealed class MfaOptions
{
    public const string SectionName = "Mfa";

    public MfaPolicy Policy { get; set; } = MfaPolicy.Optional;
    public int ChallengeLifetimeMinutes { get; set; } = 5;
    public int SetupLifetimeMinutes { get; set; } = 10;
    public int MaxFailedAttempts { get; set; } = 5;
    public int RecoveryCodeCount { get; set; } = 10;
}
