using System.Collections.Concurrent;
using System.Security.Claims;
using System.Security.Cryptography;
using System.Text.Encodings.Web;
using System.Threading.Channels;
using Application.Services.Licensing;
using Domain;
using Domain.Configs;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Microsoft.AspNetCore.Authentication;
using Microsoft.Extensions.Options;

namespace WebApi.Security;

internal static class CitadelAuthenticationSchemes
{
    public const string CompositeBearer = "CitadelBearer";
    public const string UserJwt = "UserJwt";
    public const string ServiceAccount = "ServiceAccount";
}

internal interface IServiceAccountLastUsedTracker
{
    void Track(Guid credentialId, DateTime usedAtUtc);
}

internal sealed class ServiceAccountLastUsedTracker : BackgroundService, IServiceAccountLastUsedTracker
{
    private readonly IServiceScopeFactory scopeFactory;
    private readonly ILogger<ServiceAccountLastUsedTracker> logger;
    private readonly ConcurrentDictionary<Guid, DateTime> tracked = new();
    private readonly Channel<LastUsedUpdate> updates;
    private readonly TimeSpan writeInterval;
    private readonly int capacity;

    public ServiceAccountLastUsedTracker(
        IServiceScopeFactory scopeFactory,
        IOptions<ServiceAccountOptions> options,
        ILogger<ServiceAccountLastUsedTracker> logger)
    {
        this.scopeFactory = scopeFactory;
        this.logger = logger;
        capacity = options.Value.LastUsedTrackingCapacity;
        writeInterval = TimeSpan.FromMinutes(options.Value.LastUsedWriteIntervalMinutes);
        updates = Channel.CreateBounded<LastUsedUpdate>(new BoundedChannelOptions(capacity)
        {
            FullMode = BoundedChannelFullMode.DropWrite,
            SingleReader = true,
            SingleWriter = false,
        });
    }

    public void Track(Guid credentialId, DateTime usedAtUtc)
    {
        while (tracked.TryGetValue(credentialId, out var previous))
        {
            if (usedAtUtc - previous < writeInterval)
                return;
            if (tracked.TryUpdate(credentialId, usedAtUtc, previous))
            {
                if (!updates.Writer.TryWrite(new LastUsedUpdate(credentialId, usedAtUtc)))
                    tracked.TryUpdate(credentialId, previous, usedAtUtc);
                return;
            }
        }

        if (tracked.Count >= capacity)
        {
            var staleBefore = usedAtUtc - writeInterval - writeInterval;
            foreach (var entry in tracked)
            {
                if (entry.Value < staleBefore)
                    tracked.TryRemove(entry.Key, out _);
            }
            if (tracked.Count >= capacity)
                return;
        }

        if (tracked.TryAdd(credentialId, usedAtUtc)
            && !updates.Writer.TryWrite(new LastUsedUpdate(credentialId, usedAtUtc)))
            tracked.TryRemove(new KeyValuePair<Guid, DateTime>(credentialId, usedAtUtc));
    }

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        await foreach (var update in updates.Reader.ReadAllAsync(stoppingToken))
        {
            try
            {
                await using var scope = scopeFactory.CreateAsyncScope();
                var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
                await unitOfWork.ServiceAccounts.UpdateLastUsedAsync(update.CredentialId, update.UsedAtUtc, stoppingToken);
                await unitOfWork.CommitAsync(stoppingToken);
            }
            catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested)
            {
                return;
            }
            catch (Exception exception)
            {
                tracked.TryRemove(new KeyValuePair<Guid, DateTime>(update.CredentialId, update.UsedAtUtc));
                logger.LogWarning(exception, "Unable to persist Service Account credential last-used metadata.");
            }
        }
    }

    private sealed record LastUsedUpdate(Guid CredentialId, DateTime UsedAtUtc);
}

internal sealed class ServiceAccountAuthenticationHandler(
    IOptionsMonitor<AuthenticationSchemeOptions> options,
    ILoggerFactory logger,
    UrlEncoder encoder,
    IUnitOfWork unitOfWork,
    ILicenseEntitlementService entitlementService,
    IServiceAccountLastUsedTracker lastUsedTracker,
    TimeProvider timeProvider)
    : AuthenticationHandler<AuthenticationSchemeOptions>(options, logger, encoder)
{
    protected override Task HandleChallengeAsync(AuthenticationProperties properties)
    {
        Response.StatusCode = StatusCodes.Status401Unauthorized;
        Response.Headers.WWWAuthenticate = "Bearer";
        return Task.CompletedTask;
    }

    protected override async Task<AuthenticateResult> HandleAuthenticateAsync()
    {
        var authorization = Request.Headers.Authorization.ToString();
        if (!authorization.StartsWith("Bearer ", StringComparison.OrdinalIgnoreCase))
            return AuthenticateResult.NoResult();

        var token = authorization["Bearer ".Length..];
        if (!TryParse(token, out var credentialId, out var secret))
            return AuthenticateResult.Fail("Invalid bearer credential.");

        try
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(LicenseCapability.CustomAccessControl, Context.RequestAborted);
            if (entitlement.IsFailure())
                return AuthenticateResult.Fail("Invalid bearer credential.");

            var credential = await unitOfWork.ServiceAccounts.GetCredentialAsync(credentialId, Context.RequestAborted);
            if (credential is null
                || credential.ArchivedAtUtc.HasValue
                || credential.RevokedAtUtc.HasValue
                || !credential.IsEnabled
                || (credential.ExpiresAtUtc.HasValue
                    && credential.ExpiresAtUtc.Value <= timeProvider.GetUtcNow().UtcDateTime))
            {
                return AuthenticateResult.Fail("Invalid bearer credential.");
            }

            var suppliedHash = SHA256.HashData(secret);
            var valid = credential.SecretHash.Length == suppliedHash.Length
                && CryptographicOperations.FixedTimeEquals(credential.SecretHash, suppliedHash);
            CryptographicOperations.ZeroMemory(suppliedHash);
            if (!valid)
                return AuthenticateResult.Fail("Invalid bearer credential.");

            var claims = new[]
            {
                new Claim(ClaimTypes.Name, credential.Name),
                new Claim(ClaimTypes.NameIdentifier, credential.ServiceAccountId.ToString()),
                new Claim("sub", credential.ServiceAccountId.ToString()),
                new Claim("actorId", credential.ActorId.ToString()),
                new Claim("principalType", AuthenticatedPrincipalType.ServiceAccount.ToString()),
                new Claim("credentialId", credential.CredentialId.ToString()),
            };
            var principal = new ClaimsPrincipal(new ClaimsIdentity(claims, Scheme.Name));
            lastUsedTracker.Track(credential.CredentialId, timeProvider.GetUtcNow().UtcDateTime);
            return AuthenticateResult.Success(new AuthenticationTicket(principal, Scheme.Name));
        }
        finally
        {
            CryptographicOperations.ZeroMemory(secret);
        }
    }

    internal static bool TryParse(string token, out Guid credentialId, out byte[] secret)
    {
        credentialId = Guid.Empty;
        secret = [];
        const string prefix = "cit_sa_";
        if (!token.StartsWith(prefix, StringComparison.Ordinal) || token.Length != prefix.Length + 32 + 1 + 43)
            return false;
        var separator = prefix.Length + 32;
        var credentialSpan = token.AsSpan(prefix.Length, 32);
        var secretSpan = token.AsSpan(separator + 1, 43);
        if (token[separator] != '.'
            || credentialSpan.IndexOfAnyExcept("0123456789abcdef") >= 0
            || secretSpan.IndexOfAnyExcept("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_") >= 0
            || !Guid.TryParseExact(credentialSpan, "N", out credentialId))
            return false;
        try
        {
            var encoded = token[(separator + 1)..].Replace('-', '+').Replace('_', '/');
            secret = Convert.FromBase64String(encoded + "=");
            return secret.Length == 32;
        }
        catch (FormatException)
        {
            return false;
        }
    }
}
