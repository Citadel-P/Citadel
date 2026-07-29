using Application.Services.Identity;

namespace Tests.Unit.Application.Services.Identity;

public sealed class LocalPasswordPolicyTests
{
    [Theory]
    [InlineData(null)]
    [InlineData("short")]
    [InlineData("admin123")]
    public void GetValidationError_Should_Reject_Weak_Passwords(string? password)
    {
        Assert.NotNull(LocalPasswordPolicy.GetValidationError(password));
    }

    [Fact]
    public void GetValidationError_Should_Reject_Identity_Values()
    {
        const string identityValue = "administrator-account";

        Assert.NotNull(LocalPasswordPolicy.GetValidationError(
            identityValue,
            identityValue,
            "admin@example.test"));
        Assert.NotNull(LocalPasswordPolicy.GetValidationError(
            "admin@example.test",
            "administrator",
            "admin@example.test"));
    }

    [Fact]
    public void GetValidationError_Should_Accept_Strong_Password()
    {
        Assert.Null(LocalPasswordPolicy.GetValidationError(
            "correct-horse-battery-staple",
            "admin",
            "admin@example.test"));
    }
}
