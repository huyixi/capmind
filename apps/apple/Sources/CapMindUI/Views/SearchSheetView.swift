import SwiftUI
import CapMindFeatures

public struct SearchSheetView: View {
    @ObservedObject private var viewModel: SearchViewModel

    public init(viewModel: SearchViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        NavigationStack {
            List(viewModel.state.results) { memo in
                VStack(alignment: .leading, spacing: 4) {
                    Text(memo.text)
                        .lineLimit(3)
                    Text(memo.createdAt, style: .date)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .overlay {
                if viewModel.state.results.isEmpty, !viewModel.state.query.isEmpty, !viewModel.state.isSearching {
                    ContentUnavailableView("No matching memos", systemImage: "magnifyingglass")
                }
            }
            .searchable(
                text: Binding(
                    get: { viewModel.state.query },
                    set: { value in
                        Task { await viewModel.updateQuery(value) }
                    }
                ),
                placement: .automatic,
                prompt: "Search memos"
            )
            .navigationTitle("Search")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Close") {
                        viewModel.close()
                    }
                }
            }
            .safeAreaInset(edge: .bottom) {
                if let errorMessage = viewModel.state.errorMessage {
                    Text(errorMessage)
                        .font(.caption)
                        .foregroundStyle(.red)
                        .padding(.bottom, 8)
                }
            }
        }
    }
}
