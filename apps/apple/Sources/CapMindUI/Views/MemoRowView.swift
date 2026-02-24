import SwiftUI
import CapMindCore

public struct MemoRowView: View {
    public let memo: MemoEntity

    public init(memo: MemoEntity) {
        self.memo = memo
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(memo.text)
                .font(.body)
                .lineLimit(4)

            HStack(spacing: 8) {
                Text(memo.createdAt, style: .date)
                Text(memo.updatedAt, style: .time)
                if !memo.images.isEmpty {
                    Text("\(memo.images.count) images")
                }
                Spacer()
                Text("v\(memo.version)")
            }
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        .padding(.vertical, 4)
    }
}
