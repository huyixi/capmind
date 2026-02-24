import Foundation

public protocol ImageRepository: Sendable {
    func uploadImages(userID: String, localReferences: [String]) async throws -> [String]
}
