import Foundation
import CapMindCore

public struct ComposerViewState: Equatable {
    public enum Mode: String {
        case create
        case edit
    }

    public var isPresented: Bool
    public var mode: Mode
    public var text: String
    public var imageReferences: [String]
    public var editingMemo: MemoEntity?
    public var isSubmitting: Bool
    public var errorMessage: String?

    public init(
        isPresented: Bool = false,
        mode: Mode = .create,
        text: String = "",
        imageReferences: [String] = [],
        editingMemo: MemoEntity? = nil,
        isSubmitting: Bool = false,
        errorMessage: String? = nil
    ) {
        self.isPresented = isPresented
        self.mode = mode
        self.text = text
        self.imageReferences = imageReferences
        self.editingMemo = editingMemo
        self.isSubmitting = isSubmitting
        self.errorMessage = errorMessage
    }
}
