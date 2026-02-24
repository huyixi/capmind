import Foundation
import CapMindCore

public struct SupabaseImageRepository: ImageRepository {
    private let client: any SupabaseStorageClientProtocol

    public init(client: any SupabaseStorageClientProtocol) {
        self.client = client
    }

    public func uploadImages(userID: String, localReferences: [String]) async throws -> [String] {
        try await client.uploadImages(userID: userID, localReferences: localReferences)
    }
}
