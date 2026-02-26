import SwiftUI
import CapMindFeatures
import CapMindCore

public struct ComposerSheetView: View {
    @ObservedObject private var viewModel: ComposerViewModel
    private let userID: String
    private let onSubmitted: (MemoEntity) -> Void

    public init(
        viewModel: ComposerViewModel,
        userID: String,
        onSubmitted: @escaping (MemoEntity) -> Void
    ) {
        self.viewModel = viewModel
        self.userID = userID
        self.onSubmitted = onSubmitted
    }

    public var body: some View {
        NavigationStack {
            Form {
                Section("Memo") {
                    TextEditor(text: Binding(
                        get: { viewModel.state.text },
                        set: { viewModel.updateText($0) }
                    ))
                    .frame(minHeight: 160)
                }

                Section("Image references") {
                    TextEditor(text: Binding(
                        get: { viewModel.state.imageReferences.joined(separator: "\n") },
                        set: {
                            let refs = $0
                                .split(separator: "\n")
                                .map { String($0).trimmingCharacters(in: .whitespacesAndNewlines) }
                                .filter { !$0.isEmpty }
                            viewModel.updateImageReferences(refs)
                        }
                    ))
                    .frame(minHeight: 80)

                    Text("Use local file paths or pre-uploaded URLs. One reference per line.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                if let errorMessage = viewModel.state.errorMessage {
                    Text(errorMessage)
                        .foregroundStyle(.red)
                }
            }
            .navigationTitle(viewModel.state.mode == .create ? "New memo" : "Edit memo")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        viewModel.close()
                    }
                }

                ToolbarItem(placement: .confirmationAction) {
                    Button(viewModel.state.mode == .create ? "Create" : "Save") {
                        Task {
                            if let memo = await viewModel.submit(userID: userID) {
                                onSubmitted(memo)
                                if let forkedMemo = viewModel.consumePendingForkedMemo() {
                                    onSubmitted(forkedMemo)
                                }
                                viewModel.close()
                            }
                        }
                    }
                    .disabled(viewModel.state.isSubmitting)
                }
            }
        }
    }
}
