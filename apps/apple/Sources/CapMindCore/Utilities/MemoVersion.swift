import Foundation

public enum MemoVersion {
    public static func normalize(_ value: String?) -> String {
        guard let value else { return "" }
        return value
    }

    public static func normalizeExpected(_ value: String?) -> String {
        let normalized = normalize(value)
        let allDigits = !normalized.isEmpty && normalized.unicodeScalars.allSatisfy { CharacterSet.decimalDigits.contains($0) }
        return allDigits ? normalized : "0"
    }

    public static func next(_ value: String?) -> String {
        let current = UInt64(normalizeExpected(value)) ?? 0
        return String(current + 1)
    }
}
