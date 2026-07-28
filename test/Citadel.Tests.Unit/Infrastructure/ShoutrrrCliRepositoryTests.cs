using Infrastructure.Repositories;

namespace Tests.Unit.Infrastructure;

public sealed class ShoutrrrCliRepositoryTests
{
    [Theory]
    [InlineData(1, 1)]
    [InlineData(2, 1)]
    [InlineData(8, 4)]
    [InlineData(64, 4)]
    public void GetMaxDegreeOfParallelism_ShouldRemainValidAndBounded(
        int processorCount,
        int expected)
    {
        Assert.Equal(
            expected,
            ShoutrrrCliRepository.GetMaxDegreeOfParallelism(processorCount));
    }
}
