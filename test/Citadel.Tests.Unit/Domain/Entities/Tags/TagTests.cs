using Domain.Entities.Tags;

namespace Tests.Unit.Domain.Entities.Tags;

public sealed class TagTests
{
    [Fact]
    public void Create_Should_Trim_Name_Normalize_Name_And_Uppercase_Color()
    {
        var actorId = Guid.NewGuid();

        var tag = Tag.Create(" Prod ", "#ef4444", actorId);

        Assert.Equal("Prod", tag.Name);
        Assert.Equal("prod", tag.NormalizedName);
        Assert.Equal("#EF4444", tag.Color);
        Assert.Equal(actorId, tag.CreatedByActorId);
    }

    [Theory]
    [InlineData("red")]
    [InlineData("#FFF")]
    [InlineData("22C55E")]
    public void Create_Should_Reject_Invalid_Color(string color)
    {
        Assert.Throws<ArgumentException>(() => Tag.Create("Prod", color, Guid.NewGuid()));
    }

    [Fact]
    public void Create_Should_Reject_Name_Longer_Than_Max_Length()
    {
        var name = new string('a', TagValidation.MaxNameLength + 1);

        Assert.Throws<ArgumentException>(() => Tag.Create(name, "#22C55E", Guid.NewGuid()));
    }

    [Fact]
    public void RenameAndRecolor_Should_Update_Display_Normalized_Name_And_Color()
    {
        var tag = Tag.Create("Prod", "#EF4444", Guid.NewGuid());

        var updated = tag.RenameAndRecolor(" Production ", "#dc2626");

        Assert.Equal("Production", updated.Name);
        Assert.Equal("production", updated.NormalizedName);
        Assert.Equal("#DC2626", updated.Color);
        Assert.True(updated.UpdatedAt >= tag.UpdatedAt);
    }
}
