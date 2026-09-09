using System.ComponentModel;
using System.Globalization;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Releases;

public sealed class CreateReleaseCommand : AsyncCommand<CreateReleaseCommand.Settings>
{
    public sealed class Settings : ApiSettings
    {
        [CommandOption("--project <PROJECT>")]
        [Description("Project id, slug or name")]
        public required string Project { get; init; }

        [CommandOption("--version <VERSION>")]
        [Description("Release version string, e.g. my-package@1.2.3")]
        public required string Version { get; init; }

        [CommandOption("--timestamp <TIMESTAMP>")]
        [Description("Release date as ISO 8601; defaults to now on the server")]
        public string? Timestamp { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();

        var body = new Dictionary<string, object?>
        {
            ["project"] = await client.ResolveProjectIdAsync(settings.Project),
            ["version"] = settings.Version
        };

        if (settings.Timestamp is not null)
        {
            if (!DateTimeOffset.TryParse(settings.Timestamp, CultureInfo.InvariantCulture,
                    DateTimeStyles.AssumeUniversal | DateTimeStyles.AdjustToUniversal, out var parsed))
                throw new BugsinkApiException($"Could not read '{settings.Timestamp}' as a date and time.");

            body["timestamp"] = parsed.ToString("o", CultureInfo.InvariantCulture);
        }

        YamlOutput.Write(await client.PostAsync("releases/", body));
        Status.Ok($"Release {settings.Version} created.");
        return 0;
    }
}
