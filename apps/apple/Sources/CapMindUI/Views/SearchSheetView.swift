import SwiftUI
import CapMindFeatures

public struct SearchSheetView: View {
    @ObservedObject private var viewModel: SearchViewModel

    public init(viewModel: SearchViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        NavigationStack {
            List {
                if viewModel.state.query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    if viewModel.state.recentQueries.isEmpty {
                        ContentUnavailableView(
                            "Start typing to search",
                            systemImage: "magnifyingglass"
                        )
                    } else {
                        Section("Recent searches") {
                            ForEach(viewModel.state.recentQueries, id: \.self) { query in
                                HStack(spacing: 8) {
                                    Button {
                                        Task { await viewModel.applyRecentQuery(query) }
                                    } label: {
                                        Label(query, systemImage: "clock")
                                            .foregroundStyle(.primary)
                                            .frame(maxWidth: .infinity, alignment: .leading)
                                    }
                                    .buttonStyle(.plain)

                                    Button(role: .destructive) {
                                        viewModel.removeRecentQuery(query)
                                    } label: {
                                        Image(systemName: "xmark.circle.fill")
                                    }
                                    .buttonStyle(.plain)
                                }
                            }
                        }
                    }
                } else {
                    ForEach(viewModel.state.results) { memo in
                        VStack(alignment: .leading, spacing: 4) {
                            Text(memo.text)
                                .lineLimit(3)
                            Text(memo.createdAt, style: .date)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
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
            .onSubmit(of: .search) {
                viewModel.commitCurrentQuery()
            }
            .navigationTitle("Search")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Close") {
                        viewModel.close()
                    }
                }

                ToolbarItem(placement: .primaryAction) {
                    Button("Clear") {
                        viewModel.clear()
                    }
                    .disabled(viewModel.state.query.isEmpty)
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
