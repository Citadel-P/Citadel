using Tests.Integration;

[assembly: AssemblyFixture(typeof(PostgresTestFixture))]
[assembly: CollectionBehavior(MaxParallelThreads = 8)]
