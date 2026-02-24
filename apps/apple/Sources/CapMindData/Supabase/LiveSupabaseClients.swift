import Foundation
import Supabase
import CapMindCore

private let memoImagesBucket = "memo-images"
private let memoImageURLTTLSeconds = 60 * 60
private let storageObjectPrefix = "/storage/v1/object/"

public struct LiveSupabaseAuthClient: SupabaseAuthClientProtocol {
    private let client: SupabaseClient

    public init(client: SupabaseClient) {
        self.client = client
    }

    public func signIn(email: String, password: String) async throws -> SupabaseAuthSessionPayload {
        let session = try await client.auth.signIn(email: email, password: password)
        return mapSession(session)
    }

    public func signUp(email: String, password: String, redirectURL: URL?) async throws -> SupabaseAuthSessionPayload? {
        let response = try await client.auth.signUp(
            email: email,
            password: password,
            redirectTo: redirectURL
        )

        guard let session = response.session else {
            return nil
        }

        return mapSession(session)
    }

    public func restoreSession() async throws -> SupabaseAuthSessionPayload? {
        do {
            let session = try await client.auth.session
            return mapSession(session)
        } catch let error as AuthError {
            if case .sessionMissing = error {
                return nil
            }
            throw error
        }
    }

    public func signOut() async throws {
        try await client.auth.signOut()
    }

    private func mapSession(_ session: Session) -> SupabaseAuthSessionPayload {
        SupabaseAuthSessionPayload(
            userID: session.user.id.uuidString.lowercased(),
            email: session.user.email ?? "",
            accessToken: session.accessToken,
            refreshToken: session.refreshToken,
            expiresAt: Date(timeIntervalSince1970: session.expiresAt)
        )
    }
}

public struct LiveSupabaseMemoClient: SupabaseMemoClientProtocol {
    private let client: SupabaseClient

    public init(client: SupabaseClient) {
        self.client = client
    }

    public func listMemos(page: Int, pageSize: Int, isTrash: Bool) async throws -> [MemoEntity] {
        let currentUserID = try await requireCurrentUserID()
        let from = max(0, page * pageSize)
        let to = max(from, from + pageSize - 1)

        let query = client
            .from("memos")
            .select(memoSelectFields)
            .eq("user_id", value: currentUserID)

        let filtered: PostgrestFilterBuilder
        if isTrash {
            filtered = query
                .not("deleted_at", operator: .is, value: Optional<Bool>.none)
        } else {
            filtered = query
                .is("deleted_at", value: nil)
        }

        let response: PostgrestResponse<[MemoRow]> = try await filtered
            .order(isTrash ? "deleted_at" : "created_at", ascending: false)
            .order("sort_order", ascending: true, referencedTable: "memo_images")
            .range(from: from, to: to)
            .execute()

        return try await buildMemoEntities(response.value)
    }

    public func searchMemos(query: String, limit: Int) async throws -> [MemoEntity] {
        let currentUserID = try await requireCurrentUserID()
        let normalized = query.trimmingCharacters(in: .whitespacesAndNewlines)
        if normalized.isEmpty {
            return []
        }

        let escaped = escapeLikePattern(normalized)
        let pattern = "%\(escaped)%"

        let response: PostgrestResponse<[MemoRow]> = try await client
            .from("memos")
            .select(memoSelectFields)
            .eq("user_id", value: currentUserID)
            .is("deleted_at", value: nil)
            .ilike("text", pattern: pattern)
            .order("created_at", ascending: false)
            .order("sort_order", ascending: true, referencedTable: "memo_images")
            .limit(limit)
            .execute()

        return try await buildMemoEntities(response.value)
    }

    public func createMemo(
        userID: String,
        text: String,
        imagePaths: [String],
        createdAt: Date,
        updatedAt: Date,
        clientID: String?
    ) async throws -> MemoEntity {
        let insertRow = MemoInsertRow(userID: userID, text: text)
        let created: PostgrestResponse<[MemoRow]> = try await client
            .from("memos")
            .insert(insertRow)
            .select(memoSelectFields)
            .execute()

        guard let row = created.value.first else {
            throw CapMindError.network(message: "Memo creation returned no row")
        }

        if !imagePaths.isEmpty {
            let normalized = imagePaths.compactMap { normalizeStoragePath(raw: $0, bucket: memoImagesBucket) }
            if !normalized.isEmpty {
                let payload = normalized.enumerated().map { index, path in
                    MemoImageInsertRow(memoID: row.id, url: path, sortOrder: index)
                }
                _ = try await client.from("memo_images").insert(payload).execute()
            }
        }

        if let fetched = try await fetchMemo(id: row.id, userID: userID) {
            return fetched
        }

        return try await buildMemoEntities([row]).first
            ?? MemoEntity(
                id: row.id,
                clientID: clientID,
                userID: row.userID,
                text: row.text,
                images: [],
                createdAt: row.createdAt,
                updatedAt: row.updatedAt,
                version: row.version,
                deletedAt: row.deletedAt,
                serverVersion: row.version,
                conflictType: nil
            )
    }

    public func updateMemo(
        id: String,
        userID: String,
        text: String,
        expectedVersion: String,
        imagePaths: [String]?
    ) async throws -> MemoEntity {
        let nextVersion = MemoVersion.next(expectedVersion)
        let payload = MemoUpdateRow(text: text, updatedAt: Date(), version: nextVersion)

        let response: PostgrestResponse<[MemoRow]> = try await client
            .from("memos")
            .update(payload)
            .eq("id", value: id)
            .eq("user_id", value: userID)
            .eq("version", value: expectedVersion)
            .select(memoSelectFields)
            .execute()

        if response.value.isEmpty {
            let existing = try await fetchMemo(id: id, userID: userID)
            if existing == nil {
                throw CapMindError.notFound
            }
            throw CapMindError.conflict(serverMemo: existing)
        }

        if let imagePaths {
            _ = try await client
                .from("memo_images")
                .delete()
                .eq("memo_id", value: id)
                .execute()

            let normalized = imagePaths.compactMap { normalizeStoragePath(raw: $0, bucket: memoImagesBucket) }
            if !normalized.isEmpty {
                let payload = normalized.enumerated().map { index, path in
                    MemoImageInsertRow(memoID: id, url: path, sortOrder: index)
                }
                _ = try await client.from("memo_images").insert(payload).execute()
            }
        }

        if let fetched = try await fetchMemo(id: id, userID: userID) {
            return fetched
        }

        throw CapMindError.network(message: "Updated memo not found")
    }

    public func deleteMemo(
        id: String,
        userID: String,
        expectedVersion: String,
        deletedAt: Date
    ) async throws -> MemoEntity? {
        let nextVersion = MemoVersion.next(expectedVersion)
        let payload = MemoDeleteRow(deletedAt: deletedAt, updatedAt: deletedAt, version: nextVersion)

        let response: PostgrestResponse<[MemoIDRow]> = try await client
            .from("memos")
            .update(payload)
            .eq("id", value: id)
            .eq("user_id", value: userID)
            .eq("version", value: expectedVersion)
            .select("id")
            .execute()

        if response.value.isEmpty {
            return nil
        }

        return try await fetchMemo(id: id, userID: userID)
    }

    public func restoreMemo(
        id: String,
        userID: String,
        expectedVersion: String,
        restoredAt: Date
    ) async throws -> MemoEntity? {
        let nextVersion = MemoVersion.next(expectedVersion)
        let payload = MemoRestoreRow(updatedAt: restoredAt, version: nextVersion)

        let response: PostgrestResponse<[MemoIDRow]> = try await client
            .from("memos")
            .update(payload)
            .eq("id", value: id)
            .eq("user_id", value: userID)
            .eq("version", value: expectedVersion)
            .select("id")
            .execute()

        if response.value.isEmpty {
            return nil
        }

        return try await fetchMemo(id: id, userID: userID)
    }

    public func fetchMemo(id: String, userID: String) async throws -> MemoEntity? {
        let response: PostgrestResponse<[MemoRow]> = try await client
            .from("memos")
            .select(memoSelectFields)
            .eq("id", value: id)
            .eq("user_id", value: userID)
            .limit(1)
            .execute()

        guard !response.value.isEmpty else {
            return nil
        }

        return try await buildMemoEntities(response.value).first
    }

    private func buildMemoEntities(_ rows: [MemoRow]) async throws -> [MemoEntity] {
        let rawImagePaths = rows.map { row in
            (row.memoImages ?? []).map(\.url)
        }

        let flattenedPaths = rawImagePaths.flatMap { $0 }
        let resolvedFlat = try await createSignedImageURLs(rawPaths: flattenedPaths)

        var cursor = 0
        return rows.map { row in
            let count = row.memoImages?.count ?? 0
            let images = Array(resolvedFlat[cursor..<min(resolvedFlat.count, cursor + count)])
            cursor += count

            return MemoEntity(
                id: row.id,
                clientID: nil,
                userID: row.userID,
                text: row.text,
                images: images,
                createdAt: row.createdAt,
                updatedAt: row.updatedAt,
                version: row.version,
                deletedAt: row.deletedAt,
                serverVersion: row.version,
                conflictType: nil
            )
        }
    }

    private func createSignedImageURLs(rawPaths: [String]) async throws -> [String] {
        if rawPaths.isEmpty {
            return []
        }

        var canonicalPaths: [String] = []
        var pathByIndex: [Int: String] = [:]

        for (index, raw) in rawPaths.enumerated() {
            if let path = normalizeStoragePath(raw: raw, bucket: memoImagesBucket) {
                canonicalPaths.append(path)
                pathByIndex[index] = path
            }
        }

        if canonicalPaths.isEmpty {
            return rawPaths
        }

        let signedByPath: [String: String]
        do {
            let signed = try await client.storage
                .from(memoImagesBucket)
                .createSignedURLs(paths: canonicalPaths, expiresIn: memoImageURLTTLSeconds)

            signedByPath = Dictionary(
                uniqueKeysWithValues: zip(canonicalPaths, signed.map(\.absoluteString))
            )
        } catch {
            signedByPath = [:]
        }

        return rawPaths.enumerated().map { index, raw in
            guard let path = pathByIndex[index] else {
                return raw
            }

            if let signed = signedByPath[path] {
                return signed
            }

            let publicURL = try? client.storage
                .from(memoImagesBucket)
                .getPublicURL(path: path)
                .absoluteString
            return publicURL ?? raw
        }
    }

    private func requireCurrentUserID() async throws -> String {
        if let user = client.auth.currentUser {
            return user.id.uuidString.lowercased()
        }

        do {
            let session = try await client.auth.session
            return session.user.id.uuidString.lowercased()
        } catch let error as AuthError {
            if case .sessionMissing = error {
                throw CapMindError.unauthorized
            }
            throw error
        }
    }
}

public struct LiveSupabaseStorageClient: SupabaseStorageClientProtocol {
    private let client: SupabaseClient

    public init(client: SupabaseClient) {
        self.client = client
    }

    public func uploadImages(userID: String, localReferences: [String]) async throws -> [String] {
        var uploadedPaths: [String] = []

        for reference in localReferences {
            guard let fileURL = fileURLFrom(reference: reference) else {
                continue
            }

            let ext = fileURL.pathExtension.isEmpty ? "bin" : fileURL.pathExtension
            let objectPath = "\(userID)/\(Int(Date().timeIntervalSince1970 * 1000))-\(UUID().uuidString.lowercased()).\(ext)"

            do {
                _ = try await client.storage
                    .from(memoImagesBucket)
                    .upload(objectPath, fileURL: fileURL)
                uploadedPaths.append(objectPath)
            } catch {
                // Continue uploading other files if one upload fails.
                continue
            }
        }

        return uploadedPaths
    }

    private func fileURLFrom(reference: String) -> URL? {
        let trimmed = reference.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            return nil
        }

        if trimmed.hasPrefix("file://") {
            return URL(string: trimmed)
        }

        return URL(fileURLWithPath: trimmed)
    }
}

public struct LiveSupabaseClientBundle {
    public let supabase: SupabaseClient
    public let auth: LiveSupabaseAuthClient
    public let memo: LiveSupabaseMemoClient
    public let storage: LiveSupabaseStorageClient

    public init(configuration: SupabaseConfiguration) {
        let client = SupabaseClient(
            supabaseURL: configuration.url,
            supabaseKey: configuration.anonKey
        )
        self.supabase = client
        self.auth = LiveSupabaseAuthClient(client: client)
        self.memo = LiveSupabaseMemoClient(client: client)
        self.storage = LiveSupabaseStorageClient(client: client)
    }
}

private let memoSelectFields = "id, user_id, text, created_at, updated_at, version, deleted_at, memo_images(url, sort_order)"

private struct MemoInsertRow: Encodable {
    let userID: String
    let text: String

    enum CodingKeys: String, CodingKey {
        case userID = "user_id"
        case text
    }
}

private struct MemoUpdateRow: Encodable {
    let text: String
    let updatedAt: Date
    let version: String

    enum CodingKeys: String, CodingKey {
        case text
        case updatedAt = "updated_at"
        case version
    }
}

private struct MemoDeleteRow: Encodable {
    let deletedAt: Date
    let updatedAt: Date
    let version: String

    enum CodingKeys: String, CodingKey {
        case deletedAt = "deleted_at"
        case updatedAt = "updated_at"
        case version
    }
}

private struct MemoRestoreRow: Encodable {
    let updatedAt: Date
    let version: String
    let deletedAt: Date? = nil

    enum CodingKeys: String, CodingKey {
        case updatedAt = "updated_at"
        case version
        case deletedAt = "deleted_at"
    }
}

private struct MemoImageInsertRow: Encodable {
    let memoID: String
    let url: String
    let sortOrder: Int

    enum CodingKeys: String, CodingKey {
        case memoID = "memo_id"
        case url
        case sortOrder = "sort_order"
    }
}

private struct MemoIDRow: Decodable {
    let id: String
}

private struct MemoRow: Decodable {
    let id: String
    let userID: String
    let text: String
    let createdAt: Date
    let updatedAt: Date
    let version: String
    let deletedAt: Date?
    let memoImages: [MemoImageRow]?

    enum CodingKeys: String, CodingKey {
        case id
        case userID = "user_id"
        case text
        case createdAt = "created_at"
        case updatedAt = "updated_at"
        case version
        case deletedAt = "deleted_at"
        case memoImages = "memo_images"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        userID = try container.decode(String.self, forKey: .userID)
        text = try container.decode(String.self, forKey: .text)
        createdAt = try container.decode(Date.self, forKey: .createdAt)
        updatedAt = try container.decode(Date.self, forKey: .updatedAt)

        if let versionString = try? container.decode(String.self, forKey: .version) {
            version = versionString
        } else if let versionInt = try? container.decode(Int.self, forKey: .version) {
            version = String(versionInt)
        } else if let versionDouble = try? container.decode(Double.self, forKey: .version) {
            version = String(Int(versionDouble))
        } else {
            version = ""
        }

        deletedAt = try container.decodeIfPresent(Date.self, forKey: .deletedAt)
        memoImages = try container.decodeIfPresent([MemoImageRow].self, forKey: .memoImages)
    }
}

private struct MemoImageRow: Decodable {
    let url: String
    let sortOrder: Int?

    enum CodingKeys: String, CodingKey {
        case url
        case sortOrder = "sort_order"
    }
}

private func escapeLikePattern(_ value: String) -> String {
    var escaped = ""
    for character in value {
        if character == "%" || character == "_" || character == "\\" {
            escaped.append("\\")
        }
        escaped.append(character)
    }
    return escaped
}

private func normalizeStoragePath(raw: String, bucket: String) -> String? {
    let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
    if trimmed.isEmpty {
        return nil
    }

    if trimmed.hasPrefix("data:") || trimmed.hasPrefix("blob:") {
        return nil
    }

    if !trimmed.contains("://") {
        let cleaned = trimmed.replacingOccurrences(of: "^/+", with: "", options: .regularExpression)
        let bucketPrefix = "\(bucket)/"
        let publicPrefix = "public/\(bucket)/"
        let signPrefix = "sign/\(bucket)/"

        if cleaned.hasPrefix(bucketPrefix) {
            return String(cleaned.dropFirst(bucketPrefix.count))
        }

        if cleaned.hasPrefix(publicPrefix) {
            return String(cleaned.dropFirst(publicPrefix.count))
        }

        if cleaned.hasPrefix(signPrefix) {
            return String(cleaned.dropFirst(signPrefix.count))
        }

        return cleaned
    }

    guard let url = URL(string: trimmed) else {
        return nil
    }

    let path = url.path
    guard let prefixRange = path.range(of: storageObjectPrefix) else {
        return nil
    }

    let afterPrefix = String(path[prefixRange.upperBound...])
    let segments = afterPrefix.split(separator: "/").map(String.init)
    if segments.count < 3 {
        return nil
    }

    let bucketFromURL = segments[1]
    if bucketFromURL != bucket {
        return nil
    }

    let objectPath = segments.dropFirst(2).joined(separator: "/")
    return objectPath.isEmpty ? nil : objectPath
}
