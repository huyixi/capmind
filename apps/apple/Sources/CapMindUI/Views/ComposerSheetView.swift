import SwiftUI
import CapMindFeatures
import CapMindCore
#if os(iOS)
import PhotosUI
import UniformTypeIdentifiers
#endif

public struct ComposerSheetView: View {
    @ObservedObject private var viewModel: ComposerViewModel
    private let userID: String
    private let onSubmitted: (MemoEntity) -> Void
    #if os(iOS)
    @State private var selectedPhotoItems: [PhotosPickerItem] = []
    #endif

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

                imageSection

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
            #if os(iOS)
            .onChange(of: selectedPhotoItems) { _, nextItems in
                Task { await handleSelectedPhotoItems(nextItems) }
            }
            #endif
        }
    }

    @ViewBuilder
    private var imageSection: some View {
        #if os(iOS)
        Section("Images") {
            PhotosPicker(
                selection: $selectedPhotoItems,
                maxSelectionCount: 12,
                matching: .images
            ) {
                Label("Add from Photos", systemImage: "photo.on.rectangle.angled")
            }

            if !viewModel.state.imageReferences.isEmpty {
                ScrollView(.horizontal) {
                    HStack(spacing: 12) {
                        ForEach(viewModel.state.imageReferences, id: \.self) { reference in
                            imagePreview(reference: reference)
                        }
                    }
                    .padding(.vertical, 4)
                }
                .scrollIndicators(.hidden)
            }
        }
        #else
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
        #endif
    }

    #if os(iOS)
    private func imagePreview(reference: String) -> some View {
        ZStack(alignment: .topTrailing) {
            Group {
                if let url = imageURL(for: reference) {
                    AsyncImage(url: url) { phase in
                        switch phase {
                        case .success(let image):
                            image
                                .resizable()
                                .scaledToFill()
                        default:
                            placeholderImage
                        }
                    }
                } else {
                    placeholderImage
                }
            }
            .frame(width: 84, height: 84)
            .background(.secondary.opacity(0.12))
            .clipShape(RoundedRectangle(cornerRadius: 10))

            Button {
                viewModel.removeImageReference(reference)
            } label: {
                Image(systemName: "xmark.circle.fill")
                    .symbolRenderingMode(.hierarchical)
                    .foregroundStyle(.white)
                    .background(.black.opacity(0.5))
                    .clipShape(Circle())
            }
            .buttonStyle(.plain)
            .offset(x: 8, y: -8)
        }
    }

    private var placeholderImage: some View {
        ZStack {
            Rectangle().fill(.secondary.opacity(0.2))
            Image(systemName: "photo")
                .foregroundStyle(.secondary)
        }
    }

    private func imageURL(for reference: String) -> URL? {
        let normalized = reference.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else { return nil }
        if let url = URL(string: normalized), url.scheme != nil {
            return url
        }
        return URL(fileURLWithPath: normalized)
    }

    private func handleSelectedPhotoItems(_ items: [PhotosPickerItem]) async {
        for item in items {
            guard let data = try? await item.loadTransferable(type: Data.self) else {
                continue
            }
            let ext = item.supportedContentTypes.first?.preferredFilenameExtension ?? "jpg"
            let fileURL = FileManager.default.temporaryDirectory
                .appendingPathComponent("capmind-\(UUID().uuidString.lowercased()).\(ext)")
            do {
                try data.write(to: fileURL, options: .atomic)
                viewModel.appendImageReference(fileURL.absoluteString)
            } catch {
                continue
            }
        }
        selectedPhotoItems = []
    }
    #endif
}
