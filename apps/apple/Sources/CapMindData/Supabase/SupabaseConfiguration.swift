import Foundation

public struct SupabaseConfiguration: Equatable, Sendable {
    public let url: URL
    public let anonKey: String

    public init(url: URL, anonKey: String) {
        self.url = url
        self.anonKey = anonKey
    }

    public static func fromEnvironment(_ env: [String: String] = ProcessInfo.processInfo.environment) -> SupabaseConfiguration? {
        guard let rawURL = env["SUPABASE_URL"] ?? env["NEXT_PUBLIC_SUPABASE_URL"],
              let url = URL(string: rawURL),
              let anonKey = env["SUPABASE_ANON_KEY"] ?? env["NEXT_PUBLIC_SUPABASE_ANON_KEY"],
              !anonKey.isEmpty
        else {
            return nil
        }

        return SupabaseConfiguration(url: url, anonKey: anonKey)
    }
}
