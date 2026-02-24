import SwiftUI
import CapMindCore
import CapMindFeatures

public struct MemoHomeView: View {
    @ObservedObject private var authViewModel: AuthViewModel
    @ObservedObject private var listViewModel: MemoListViewModel
    @ObservedObject private var composerViewModel: ComposerViewModel
    @ObservedObject private var searchViewModel: SearchViewModel

    public init(
        authViewModel: AuthViewModel,
        listViewModel: MemoListViewModel,
        composerViewModel: ComposerViewModel,
        searchViewModel: SearchViewModel
    ) {
        self.authViewModel = authViewModel
        self.listViewModel = listViewModel
        self.composerViewModel = composerViewModel
        self.searchViewModel = searchViewModel
    }

    public var body: some View {
        NavigationStack {
            List {
                ForEach(listViewModel.state.memos) { memo in
                    MemoRowView(memo: memo)
                        .contentShape(Rectangle())
                        .onTapGesture {
                            composerViewModel.openEdit(memo: memo)
                        }
                        .task {
                            await listViewModel.loadMoreIfNeeded(currentMemo: memo)
                        }
                        .swipeActions(edge: .trailing) {
                            if listViewModel.state.isTrashActive {
                                Button("Restore") {
                                    Task { await listViewModel.restoreMemo(memo) }
                                }
                                .tint(.green)
                            } else {
                                Button("Delete", role: .destructive) {
                                    Task { await listViewModel.deleteMemo(memo) }
                                }
                            }
                        }
                }
            }
            .overlay {
                if listViewModel.state.isLoadingInitial {
                    ProgressView("Loading memos...")
                } else if listViewModel.state.memos.isEmpty {
                    ContentUnavailableView(
                        listViewModel.state.isTrashActive ? "No trashed memos" : "No memos",
                        systemImage: listViewModel.state.isTrashActive ? "trash" : "note.text.badge.plus"
                    )
                }
            }
            .navigationTitle(listViewModel.state.isTrashActive ? "Trash" : "CapMind")
            .toolbar {
                ToolbarItemGroup(placement: .automatic) {
                    Button(listViewModel.state.isTrashActive ? "Show Active" : "Show Trash") {
                        Task { await listViewModel.setTrashActive(!listViewModel.state.isTrashActive) }
                    }

                    Button("Search") {
                        searchViewModel.open()
                    }
                }

                ToolbarItemGroup(placement: .automatic) {
                    Button("Sync") {
                        Task { _ = await listViewModel.syncNow() }
                    }

                    Button("Refresh") {
                        Task { await listViewModel.refresh() }
                    }

                    Button("Sign out") {
                        Task { await authViewModel.signOut() }
                    }
                }
            }
            .safeAreaInset(edge: .bottom, alignment: .trailing) {
                Button {
                    composerViewModel.openCreate()
                } label: {
                    Label("New memo", systemImage: "plus")
                        .labelStyle(.iconOnly)
                        .font(.title2)
                        .padding(14)
                        .background(.blue)
                        .foregroundStyle(.white)
                        .clipShape(Circle())
                        .shadow(radius: 3)
                }
                .padding(.trailing, 20)
                .padding(.bottom, 12)
            }
            .task(id: authViewModel.state.session?.userID) {
                if let session = authViewModel.state.session {
                    await listViewModel.loadInitial(userID: session.userID)
                }
            }
            .sheet(isPresented: Binding(
                get: { composerViewModel.state.isPresented },
                set: { if !$0 { composerViewModel.close() } }
            )) {
                if let session = authViewModel.state.session {
                    ComposerSheetView(
                        viewModel: composerViewModel,
                        userID: session.userID,
                        onSubmitted: { memo in
                            listViewModel.upsertMemo(memo)
                        }
                    )
                }
            }
            .sheet(isPresented: Binding(
                get: { searchViewModel.state.isPresented },
                set: { if !$0 { searchViewModel.close() } }
            )) {
                SearchSheetView(viewModel: searchViewModel)
            }
            .safeAreaInset(edge: .bottom) {
                if let errorMessage = listViewModel.state.errorMessage {
                    Text(errorMessage)
                        .font(.caption)
                        .foregroundStyle(.red)
                        .padding(.bottom, 4)
                }
            }
        }
    }
}
