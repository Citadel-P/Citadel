using Application.Services.Identity;

namespace Tests.Unit.Application.Services.Identity;

public sealed class CitadelPasswordHasherTests
{
    private readonly CitadelPasswordHasher hasher = new();

    [Fact]
    public void Hash_Should_RoundTrip_With_Current_Format()
    {
        const string password = "correct-horse-battery-staple";

        var hash = hasher.Hash(password);

        Assert.NotEqual(password, hash);
        Assert.True(hasher.Verify(password, hash));
        Assert.False(hasher.Verify("wrong-password", hash));
    }

    [Theory]
    [InlineData("")]
    [InlineData("not-base64")]
    [InlineData("AA==")]
    public void Verify_Should_Reject_Malformed_Hashes(string hash)
    {
        Assert.False(hasher.Verify("any-password", hash));
    }
}
