import Foundation

public enum CapMindError: Error, Equatable, Sendable {
    case unauthorized
    case notFound
    case conflict(serverMemo: MemoEntity?)
    case network(message: String)
    case invalidInput(message: String)
    case unsupported(message: String)
}
