import Foundation
import CapMindCore

public struct InMemoryImageRepository: ImageRepository {
    public init() {}

    public func uploadImages(userID: String, localReferences: [String]) async throws -> [String] {
        localReferences.enumerated().map { index, ref in
            "mock://memo-images/\(userID)/\(index)-\(ref.hashValue)"
        }
    }
}
