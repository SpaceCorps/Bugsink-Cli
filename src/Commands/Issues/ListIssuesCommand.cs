using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Issues;

public sealed class ListIssuesCommand : AsyncCommand<ListIssuesCommand.Settings>
{
    public sealed class Settings : ProjectSettings
    {
        [CommandOption("--sort <SORT>")]
        [Description("digest_order, digested_event_count or last_seen; defaults to digest_order")]
        public string? Sort { get; init; }

        [CommandOption("--order <ORDER>")]
        [Description("asc or desc; defaults to asc")]
        public string? Order { get; init; }

        [CommandOption("--status <STATUS>")]
        [Description("Filter by state: all, open, unresolved, resolved or muted; defaults to all")]
        public string Status { get; init; } = "all";
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        var projectId = await client.ResolveProjectIdAsync(settings.Project);

        var path = $"issues/?project={projectId}";
        if (settings.Sort is not null) path += $"&sort={Uri.EscapeDataString(settings.Sort)}";
        if (settings.Order is not null) path += $"&order={Uri.EscapeDataString(settings.Order)}";

        // The API has no state filter, so states are matched here as pages arrive.
        var keep = BuildFilter(settings.Status);
        YamlOutput.Write(await client.GetPagedAsync(path, settings.EffectiveLimit, keep));
        return 0;
    }

    private static Func<Dictionary<string, object?>, bool>? BuildFilter(string status) =>
        status.ToLowerInvariant() switch
        {
            "all" => null,
            "open" => issue => !Flag(issue, "is_resolved") && !Flag(issue, "is_muted"),
            "unresolved" => issue => !Flag(issue, "is_resolved"),
            "resolved" => issue => Flag(issue, "is_resolved"),
            "muted" => issue => Flag(issue, "is_muted"),
            _ => throw new BugsinkApiException(
                $"Unknown --status '{status}'. Use all, open, unresolved, resolved or muted.")
        };

    private static bool Flag(Dictionary<string, object?> issue, string key) =>
        issue.TryGetValue(key, out var value) && value is true;
}
