using System.ComponentModel;
using Spectre.Console.Cli;

namespace Bugsink.Console.Infrastructure;

public class ApiSettings : CommandSettings
{
    [CommandOption("--api-token <TOKEN>")]
    [Description("Bugsink API token (or set BUGSINK_API_TOKEN env var)")]
    public string? ApiToken { get; init; }

    [CommandOption("--endpoint <URL>")]
    [Description("Bugsink base URL, e.g. https://bugsink.example.com (or set BUGSINK_ENDPOINT env var)")]
    public string? Endpoint { get; init; }

    public BugsinkClient CreateClient()
    {
        var token = ApiToken ?? Environment.GetEnvironmentVariable("BUGSINK_API_TOKEN")
            ?? throw new BugsinkApiException(
                "API token required. Use --api-token or set BUGSINK_API_TOKEN.");
        var endpoint = Endpoint ?? Environment.GetEnvironmentVariable("BUGSINK_ENDPOINT")
            ?? throw new BugsinkApiException(
                "Bugsink URL required. Use --endpoint or set BUGSINK_ENDPOINT.");
        return new BugsinkClient(token, endpoint);
    }
}

/// <summary>Settings for a command that pages through a list endpoint.</summary>
public class PagedSettings : ApiSettings
{
    [CommandOption("--limit <N>")]
    [Description("Maximum number of items to return; default 50")]
    public int Limit { get; init; } = 50;

    [CommandOption("--all")]
    [Description("Return every item, following pagination to the end")]
    public bool All { get; init; }

    public int? EffectiveLimit => All ? null : Limit;
}

/// <summary>Settings for any command that operates on a single project.</summary>
public class ProjectSettings : PagedSettings
{
    [CommandOption("--project <PROJECT>")]
    [Description("Project id, slug or name")]
    public required string Project { get; init; }
}
