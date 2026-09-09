using System.Net;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;

namespace Bugsink.Console.Infrastructure;

public sealed class BugsinkClient
{
    /// <summary>The Bugsink-specific API; the Sentry-compatible ingest paths live elsewhere.</summary>
    private const string ApiPath = "api/canonical/0/";

    private readonly HttpClient _http;

    public BugsinkClient(string apiToken, string baseUrl)
    {
        var root = baseUrl.TrimEnd('/');
        _http = new HttpClient { BaseAddress = new Uri($"{root}/{ApiPath}") };
        _http.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", apiToken);
        _http.DefaultRequestHeaders.Accept.Add(new MediaTypeWithQualityHeaderValue("application/json"));
    }

    public Task<object?> GetAsync(string path) => SendAsync(HttpMethod.Get, path, null);

    public Task<object?> PostAsync(string path, object? body = null) => SendAsync(HttpMethod.Post, path, body);

    public Task<object?> PatchAsync(string path, object body) => SendAsync(HttpMethod.Patch, path, body);

    public Task<object?> DeleteAsync(string path) => SendAsync(HttpMethod.Delete, path, null);

    /// <summary>
    /// Reads a non-JSON endpoint, such as the Markdown stacktrace renderer. Content negotiation is
    /// strict there: the default JSON Accept header earns a 406.
    /// </summary>
    public async Task<string> GetTextAsync(string path)
    {
        using var request = new HttpRequestMessage(HttpMethod.Get, path);
        request.Headers.Accept.Clear();
        request.Headers.Accept.Add(new MediaTypeWithQualityHeaderValue("text/markdown"));
        request.Headers.Accept.Add(new MediaTypeWithQualityHeaderValue("text/plain", 0.9));
        request.Headers.Accept.Add(new MediaTypeWithQualityHeaderValue("*/*", 0.8));

        using var response = await _http.SendAsync(request);
        var text = await response.Content.ReadAsStringAsync();
        if (!response.IsSuccessStatusCode)
            throw new BugsinkApiException(FormatFailure(response.StatusCode, response.ReasonPhrase, text));
        return text;
    }

    /// <summary>
    /// Walks the cursor pagination and returns a flat list. <paramref name="limit"/> is the number of
    /// items to return; null means every page. <paramref name="keep"/> filters items that server-side
    /// filtering cannot express, and is applied before the limit is counted.
    /// </summary>
    public async Task<List<object?>> GetPagedAsync(
        string path, int? limit, Func<Dictionary<string, object?>, bool>? keep = null)
    {
        var results = new List<object?>();
        string? cursor = null;

        while (true)
        {
            var url = cursor is null
                ? path
                : $"{path}{(path.Contains('?') ? '&' : '?')}cursor={Uri.EscapeDataString(cursor)}";

            if (await GetAsync(url) is not Dictionary<string, object?> page)
                break;

            if (page.TryGetValue("results", out var raw) && raw is List<object?> items)
            {
                foreach (var item in items)
                {
                    if (keep is not null && item is Dictionary<string, object?> row && !keep(row))
                        continue;

                    results.Add(item);
                    if (limit is not null && results.Count >= limit)
                        return results;
                }
            }

            cursor = ExtractCursor(page.TryGetValue("next", out var next) ? next as string : null);
            if (cursor is null)
                break;
        }

        return results;
    }

    /// <summary>
    /// The next link is an absolute URL built from the request the server saw, which behind a
    /// reverse proxy may not be reachable as-is. Only the cursor matters, so that is all we keep.
    /// </summary>
    private static string? ExtractCursor(string? nextUrl)
    {
        if (string.IsNullOrWhiteSpace(nextUrl)) return null;

        var query = nextUrl.Contains('?') ? nextUrl[(nextUrl.IndexOf('?') + 1)..] : nextUrl;
        foreach (var pair in query.Split('&', StringSplitOptions.RemoveEmptyEntries))
        {
            var parts = pair.Split('=', 2);
            if (parts.Length == 2 && parts[0] == "cursor")
                return Uri.UnescapeDataString(parts[1]);
        }

        return null;
    }

    private async Task<object?> SendAsync(HttpMethod method, string path, object? body)
    {
        using var request = new HttpRequestMessage(method, path);
        if (body is not null)
            request.Content = new StringContent(JsonSerializer.Serialize(body), Encoding.UTF8, "application/json");

        using var response = await _http.SendAsync(request);
        var text = await response.Content.ReadAsStringAsync();

        if (string.IsNullOrWhiteSpace(text))
        {
            if (!response.IsSuccessStatusCode)
                throw new BugsinkApiException(FormatFailure(response.StatusCode, response.ReasonPhrase, text));
            return null;
        }

        JsonDocument document;
        try
        {
            document = JsonDocument.Parse(text);
        }
        catch (JsonException)
        {
            if (!response.IsSuccessStatusCode)
                throw new BugsinkApiException(FormatFailure(response.StatusCode, response.ReasonPhrase, text));
            return text;
        }

        using (document)
        {
            if (!response.IsSuccessStatusCode)
                throw new BugsinkApiException(
                    FormatFailure(response.StatusCode, response.ReasonPhrase, text, document.RootElement));

            return ToObject(document.RootElement);
        }
    }

    /// <summary>
    /// Django REST Framework reports errors either as a detail string or as a map of field name to a
    /// list of messages. Both are flattened onto one line so a caller sees the cause, not the JSON.
    /// </summary>
    private static string FormatFailure(HttpStatusCode status, string? reason, string body, JsonElement? parsed = null)
    {
        var prefix = $"{(int)status} {reason}";

        if (parsed is { ValueKind: JsonValueKind.Object } root)
        {
            if (root.TryGetProperty("detail", out var detail) && detail.ValueKind == JsonValueKind.String)
                return $"{prefix}: {detail.GetString()}";

            var fields = root.EnumerateObject()
                .Select(p => $"{p.Name}: {DescribeValue(p.Value)}")
                .ToList();
            if (fields.Count > 0)
                return $"{prefix}: {string.Join("; ", fields)}";
        }

        return string.IsNullOrWhiteSpace(body) ? prefix : $"{prefix}: {body}";
    }

    private static string DescribeValue(JsonElement value) => value.ValueKind switch
    {
        JsonValueKind.Array => string.Join(" ", value.EnumerateArray().Select(DescribeValue)),
        JsonValueKind.String => value.GetString() ?? "",
        _ => value.ToString()
    };

    /// <summary>Converts JSON into plain CLR types so YamlDotNet can serialize the result.</summary>
    private static object? ToObject(JsonElement element) => element.ValueKind switch
    {
        JsonValueKind.Object => element.EnumerateObject()
            .ToDictionary(p => p.Name, p => ToObject(p.Value)),
        JsonValueKind.Array => element.EnumerateArray().Select(ToObject).ToList(),
        JsonValueKind.String => element.GetString(),
        // Boxed explicitly: a conditional would widen the long branch to double and turn every id
        // into a floating point number.
        JsonValueKind.Number => element.TryGetInt64(out var l) ? l : (object)element.GetDouble(),
        JsonValueKind.True => true,
        JsonValueKind.False => false,
        _ => null
    };

    /// <summary>
    /// Accepts a numeric project id, a slug or a project name. Slugs and names are what a human (or
    /// an agent reading a DSN) has to hand; the API itself only takes the integer id.
    /// </summary>
    public async Task<long> ResolveProjectIdAsync(string project)
    {
        if (long.TryParse(project, out var id))
            return id;

        var matches = new List<Dictionary<string, object?>>();
        foreach (var item in await GetPagedAsync("projects/", null))
        {
            if (item is not Dictionary<string, object?> row) continue;
            if (Matches(row, "slug", project) || Matches(row, "name", project))
                matches.Add(row);
        }

        if (matches.Count == 0)
            throw new BugsinkApiException($"No project found matching '{project}'.");
        if (matches.Count > 1)
            throw new BugsinkApiException(
                $"'{project}' matches {matches.Count} projects. Use the project id instead.");

        return matches[0].TryGetValue("id", out var value) && value is long projectId
            ? projectId
            : throw new BugsinkApiException($"Project '{project}' has no id.");
    }

    /// <summary>Accepts a team UUID or a team name.</summary>
    public async Task<string> ResolveTeamIdAsync(string team)
    {
        if (Guid.TryParse(team, out var guid))
            return guid.ToString();

        var matches = new List<Dictionary<string, object?>>();
        foreach (var item in await GetPagedAsync("teams/", null))
        {
            if (item is Dictionary<string, object?> row && Matches(row, "name", team))
                matches.Add(row);
        }

        if (matches.Count == 0)
            throw new BugsinkApiException($"No team found named '{team}'.");
        if (matches.Count > 1)
            throw new BugsinkApiException($"'{team}' matches {matches.Count} teams. Use the team UUID instead.");

        return matches[0].TryGetValue("id", out var value) && value is string teamId
            ? teamId
            : throw new BugsinkApiException($"Team '{team}' has no id.");
    }

    /// <summary>Returns the UUID of the most recent event on an issue, for stacktrace lookups.</summary>
    public async Task<string> ResolveLatestEventIdAsync(string issue)
    {
        var events = await GetPagedAsync($"events/?issue={Uri.EscapeDataString(issue)}&order=desc", 1);
        if (events.Count == 0)
            throw new BugsinkApiException($"Issue '{issue}' has no stored events.");

        return events[0] is Dictionary<string, object?> row &&
               row.TryGetValue("id", out var value) && value is string eventId
            ? eventId
            : throw new BugsinkApiException($"Issue '{issue}' has no stored events.");
    }

    private static bool Matches(Dictionary<string, object?> row, string key, string value) =>
        row.TryGetValue(key, out var actual) && actual is string text &&
        string.Equals(text, value, StringComparison.OrdinalIgnoreCase);
}
