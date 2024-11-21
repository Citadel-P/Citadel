namespace Infrastructure;

public partial interface IAgentProxy
{
    HttpClient Client { get; }

    IAgentProxy ForAddress(string address)
    {
        Client.BaseAddress = new Uri("http://" + address);
        return this;
    }
}